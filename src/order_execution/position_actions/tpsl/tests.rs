use super::*;
use crate::account::{AccountData, AccountDataCompleteness, UserFeeRates};
use crate::account_state::position_tpsl::{PositionTpslForm, PositionTpslSubmission};
use crate::api::ExchangeSymbol;
use crate::canvas_state::WorkspaceId;
use crate::config::{AccountProfile, KeroseneConfig};
use crate::message::Message;
use serde_json::json;

const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.connected_address = Some(ADDRESS.into());
    terminal.wallet_address_input = ADDRESS.into();
    terminal.accounts = vec![AccountProfile {
        master_address: None,
        secret_id: "test-tpsl".into(),
        name: "Test".into(),
        wallet_address: ADDRESS.into(),
        agent_key: "".to_string().into(),
        hydromancer_api_key: "".to_string().into(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.exchange_symbols = vec![ExchangeSymbol {
        key: "BTC".into(),
        ticker: "BTC".into(),
        category: "crypto".into(),
        display_name: None,
        keywords: vec![],
        asset_index: 7,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    }];
    terminal.set_account_data_for_address_for_test(ADDRESS, AccountData {
        fetch_scope: Default::default(), request_weight_estimate: 0, account_abstraction: Default::default(),
        clearinghouse: serde_json::from_value(json!({
            "marginSummary": {"accountValue":"100", "totalNtlPos":"100", "totalMarginUsed":"10"},
            "withdrawable":"90", "assetPositions":[{"position": {
                "coin":"BTC", "szi":"1", "entryPx":"100", "positionValue":"100",
                "unrealizedPnl":"0", "marginUsed":"10", "leverage":{"type":"cross", "value":10}
            }}]
        })).expect("clearinghouse fixture"),
        clearinghouses_by_dex: Default::default(),
        spot: serde_json::from_value(json!({"balances":[]})).expect("spot fixture"),
        open_orders: vec![], fills: vec![], funding_history: vec![], fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(), fetched_at_ms: TradingTerminal::now_ms(),
    });
    terminal.account_loading = false;
    terminal.position_tpsl.form = Some(PositionTpslForm {
        workspace: WorkspaceId::Main,
        visible: true,
        account_address: ADDRESS.into(),
        coin: "BTC".into(),
        is_long: true,
        take_profit: "110".into(),
        stop_loss: "90".into(),
        status: None,
    });
    terminal
}

fn pending(terminal: &mut TradingTerminal, id: u64) {
    let orders = terminal.prepare_position_tpsl().expect("valid plan");
    terminal.position_tpsl.pending = Some(PositionTpslSubmission {
        id,
        account_address: ADDRESS.into(),
        outcomes: vec![TpslOutcome::Unknown; orders.len()],
        orders,
        in_flight: true,
    });
}

fn response(statuses: serde_json::Value) -> Result<ExchangeResponse, String> {
    Ok(serde_json::from_value(
        json!({"status":"ok", "response":{"type":"order", "data":{"statuses":statuses}}}),
    )
    .expect("response fixture"))
}

#[test]
fn tpsl_prices_and_sides_for_longs_and_shorts() {
    let mut terminal = terminal();
    let orders = terminal.prepare_position_tpsl().expect("long plan");
    assert_eq!(orders.len(), 2);
    assert!(orders.iter().all(|order| !order.is_buy && order.asset == 7));
    assert_eq!(orders[0].price, "99");
    assert_eq!(orders[1].price, "81");
    assert_ne!(orders[0].cloid, orders[1].cloid);
    assert!(orders.iter().all(|order| order.cloid.len() == 34));
    assert!(validate_tpsl_trigger_sides(&orders, 100.0).is_ok());
    assert!(validate_tpsl_trigger_sides(&orders, 110.0).is_err());
    assert!(validate_tpsl_trigger_sides(&orders, 90.0).is_err());
    let data = terminal.account_data.as_mut().expect("account");
    data.clearinghouse.asset_positions[0].position.szi = "-2".into();
    let form = terminal.position_tpsl.form.as_mut().expect("form");
    form.is_long = false;
    form.take_profit = "90".into();
    form.stop_loss = "110".into();
    let orders = terminal.prepare_position_tpsl().expect("short plan");
    assert!(orders.iter().all(|order| order.is_buy));
    assert_eq!(orders[0].price, "99");
    assert_eq!(orders[1].price, "121");
    assert!(validate_tpsl_trigger_sides(&orders, 100.0).is_ok());
    assert!(validate_tpsl_trigger_sides(&orders, 90.0).is_err());
    assert!(validate_tpsl_trigger_sides(&orders, 110.0).is_err());
}

#[test]
fn tpsl_rejects_invalid_prices_precision_and_empty_form() {
    for value in ["NaN", "inf", "-1", "0", "", "abc", "1e309", "110.001"] {
        assert!(tpsl_prices(value, 4, false).is_err(), "{value}");
    }
    assert_eq!(
        tpsl_prices(" 110.01 ", 4, true).expect("valid price"),
        ("110.01".into(), "121.01".into())
    );
    let mut terminal = terminal();
    let form = terminal.position_tpsl.form.as_mut().expect("form");
    form.take_profit.clear();
    assert_eq!(terminal.prepare_position_tpsl().expect("SL only").len(), 1);
    terminal
        .position_tpsl
        .form
        .as_mut()
        .expect("form")
        .stop_loss
        .clear();
    assert!(terminal.prepare_position_tpsl().is_err());
}

#[test]
fn tpsl_requires_fresh_complete_owned_account_and_unchanged_position_direction() {
    for case in 0..10 {
        let mut terminal = terminal();
        match case {
            0 => terminal.account_data.as_mut().expect("data").fetched_at_ms = 1,
            1 => terminal.account_loading = true,
            2 => terminal.account_reconciliation_required = true,
            3 => {
                terminal
                    .account_data
                    .as_mut()
                    .expect("data")
                    .completeness
                    .positions_actionable = false
            }
            4 => {
                terminal
                    .account_data
                    .as_mut()
                    .expect("data")
                    .completeness
                    .open_orders_complete = false
            }
            5 => terminal.connected_address = Some("different-account".into()),
            6 => terminal.account_data_address = Some("different-account".into()),
            7 => {
                terminal
                    .account_data
                    .as_mut()
                    .expect("data")
                    .clearinghouse
                    .asset_positions[0]
                    .position
                    .szi = "-1".into()
            }
            8 => terminal
                .account_data
                .as_mut()
                .expect("data")
                .clearinghouse
                .asset_positions
                .clear(),
            _ => terminal.exchange_symbols[0].market_type = MarketType::Spot,
        }
        assert!(terminal.prepare_position_tpsl().is_err(), "case {case}");
    }
}

#[test]
fn tpsl_existing_trigger_blocks_duplicate_kind_but_allows_other_kind() {
    let mut terminal = terminal();
    terminal
        .account_data
        .as_mut()
        .expect("data")
        .open_orders
        .push(
        serde_json::from_value(json!({
            "coin":"BTC", "side":"A", "limitPx":"99", "sz":"0", "oid":42, "timestamp":1,
            "reduceOnly":true, "isTrigger":true, "orderType":"Take Profit Market", "triggerPx":"110"
        }))
        .expect("trigger fixture"),
    );
    assert!(
        terminal
            .prepare_position_tpsl()
            .expect_err("duplicate TP")
            .contains("already exists")
    );
    terminal
        .position_tpsl
        .form
        .as_mut()
        .expect("form")
        .take_profit
        .clear();
    assert_eq!(terminal.prepare_position_tpsl().expect("new SL").len(), 1);
    terminal.account_data.as_mut().expect("data").open_orders[0].order_type = None;
    assert!(terminal.prepare_position_tpsl().is_err());
}

#[test]
fn tpsl_response_parsing_preserves_partial_failure_and_uncertainty() {
    assert_eq!(
        position_tpsl_outcomes(response(json!([{"error":"Rejected"}])), 2),
        vec![TpslOutcome::Unknown; 2]
    );
    assert_eq!(
        position_tpsl_outcomes(
            response(json!([{"resting":{"oid":42}}, {"error":"Rejected"}])),
            2
        ),
        vec![
            TpslOutcome::Accepted,
            TpslOutcome::Rejected("Rejected".into())
        ]
    );
    for result in [
        Err("timeout".into()),
        response(json!([])),
        response(json!(["success"])),
        response(json!([{"resting":{"oid":0}}])),
        response(json!([{"resting":{"oid":42},"error":"inconsistent"}])),
    ] {
        assert_eq!(
            position_tpsl_outcomes(result, 1),
            vec![TpslOutcome::Unknown]
        );
    }
    assert!(matches!(
        position_tpsl_outcomes(Ok(ExchangeResponse::local_rejection("mark unavailable")), 2)
            .as_slice(),
        [TpslOutcome::Rejected(_), TpslOutcome::Rejected(_)]
    ));
}

#[test]
fn tpsl_submission_gates_double_clicks_and_has_no_optimistic_position_delta() {
    let mut terminal = terminal();
    let _task = terminal.submit_position_tpsl();
    let pending = terminal.position_tpsl.pending.as_ref().expect("submission");
    let id = pending.id;
    let cloid = pending.orders[0].cloid.clone();
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.optimistic_position_deltas().is_empty());
    let _second = terminal.submit_position_tpsl();
    let pending = terminal
        .position_tpsl
        .pending
        .as_ref()
        .expect("same submission");
    assert_eq!(pending.id, id);
    assert_eq!(pending.orders[0].cloid, cloid);
}

#[test]
fn tpsl_result_clears_only_accepted_inputs_and_refreshes_account() {
    let mut terminal = terminal();
    pending(&mut terminal, 1);
    let _task = terminal.handle_position_tpsl_result(
        1,
        response(json!([{"resting":{"oid":42}}, {"error":"Rejected"}])),
    );
    assert!(terminal.position_tpsl.pending.is_none());
    let form = terminal.position_tpsl.form.as_ref().expect("form");
    assert!(form.take_profit.is_empty());
    assert_eq!(form.stop_loss, "90");
    let (status, error) = form.status.as_ref().expect("status");
    assert!(*error && status.contains("Take profit accepted") && status.contains("Rejected"));
    assert!(terminal.account_loading);
}

#[test]
fn tpsl_late_results_do_not_mutate_another_account_or_submission() {
    let mut terminal = terminal();
    pending(&mut terminal, 7);
    let _task = terminal.handle_position_tpsl_result(
        6,
        response(json!([{"error":"Rejected"}, {"error":"Rejected"}])),
    );
    assert!(terminal.position_tpsl.pending.is_some());
    terminal.connected_address = Some("different-account".into());
    let _task = terminal.handle_position_tpsl_checked(7, vec![TpslOutcome::Accepted; 2]);
    assert!(terminal.position_tpsl.pending.is_some());
    assert!(
        terminal
            .position_tpsl
            .form
            .as_ref()
            .expect("form")
            .status
            .is_none()
    );
    terminal.clear_account_scoped_chart_state();
    assert!(terminal.position_tpsl.pending.is_none());
    assert!(terminal.position_tpsl.form.is_none());
}

#[test]
fn tpsl_uncertainty_survives_modal_close_and_can_be_checked_without_resubmitting() {
    let mut terminal = terminal();
    pending(&mut terminal, 1);
    let _task = terminal.handle_position_tpsl_result(1, Err("timeout".into()));
    assert!(
        terminal
            .position_tpsl
            .pending
            .as_ref()
            .expect("pending")
            .in_flight
    );
    let _task =
        terminal.handle_position_tpsl_checked(1, vec![TpslOutcome::Accepted, TpslOutcome::Unknown]);
    assert!(
        !terminal
            .position_tpsl
            .pending
            .as_ref()
            .expect("pending")
            .in_flight
    );
    let _task = terminal.update_order(Message::ClosePositionTpsl);
    assert!(terminal.has_pending_trading_request());
    assert!(!terminal.position_tpsl.form.as_ref().expect("form").visible);
    let _task = terminal.open_position_tpsl(WorkspaceId::Canvas(8), "ETH".into());
    let form = terminal
        .position_tpsl
        .form
        .as_ref()
        .expect("pending form restored");
    assert_eq!(form.coin, "BTC");
    assert_eq!(form.workspace, WorkspaceId::Canvas(8));
    assert!(form.visible);
    let _task = terminal.handle_position_tpsl_checked(1, vec![TpslOutcome::Accepted; 2]);
    assert!(terminal.position_tpsl.pending.is_none());
}

#[test]
fn tpsl_modal_shows_cancel_failures() {
    let mut terminal = terminal();
    let _task = terminal.update_order(Message::CancelResult {
        account_address: ADDRESS.into(),
        pending_indicator_id: None,
        result: Box::new(Ok(ExchangeResponse::local_rejection("Cancel rejected"))),
    });
    let status = terminal
        .position_tpsl
        .form
        .as_ref()
        .expect("form")
        .status
        .as_ref()
        .expect("visible cancel error");
    assert!(status.1 && status.0.contains("Cancel rejected"));
}
