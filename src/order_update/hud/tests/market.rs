use super::*;
use crate::account::{AccountData, ClearinghouseState, MarginSummary, SpotClearinghouseState};
use crate::order_pending_indicators::PendingOrderIndicatorKind;

fn market_request() -> HudOrderRequest {
    HudOrderRequest {
        order_type: HudOrderType::Market,
        ..hud_request(ChartSurfaceId::Docked(1))
    }
}

fn market_context() -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: "0x00000000000000000000000000000003".to_string(),
        surface: OrderSurface::Hud,
        symbol_key: "BTC".to_string(),
        order_kind: ExchangeOrderKind::Market,
    }
}

fn order_response(status: serde_json::Value) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {"type": "order", "data": {"statuses": [status]}}
    }))
    .expect("test order response")
}

#[test]
fn confirmed_market_fill_allows_next_click_during_refresh() {
    let mut terminal = terminal_with_hud_chart(true);
    make_btc_tradeable(&mut terminal);
    let _placement = terminal.handle_submit_hud_order(market_request());
    let indicator_id = *terminal
        .pending_order_indicators
        .keys()
        .next()
        .expect("pending market indicator");

    // Serialization still blocks another click until the exchange responds.
    let _overlapping = terminal.handle_submit_hud_order(market_request());
    assert_eq!(terminal.pending_order_indicators.len(), 1);
    assert_eq!(
        order_status_of(&terminal),
        Some((
            "Wait for pending trading requests to finish before placing a HUD order",
            true
        ))
    );

    let _refresh = terminal.handle_hud_order_result(
        Some(indicator_id),
        None,
        market_context(),
        Ok(order_response(serde_json::json!({
            "filled": {"oid": 42, "totalSz": "1", "avgPx": "100"}
        }))),
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);

    let mut next = market_request();
    next.market_side = HudOrderSide::Short;
    next.quantity = "0.25".to_string();
    let _placement = terminal.handle_submit_hud_order(next);
    assert_eq!(
        order_status_of(&terminal),
        Some(("Placing HUD market SHORT 0.25 BTC...", false))
    );
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Sell)
    );
    assert_eq!(terminal.pending_order_indicators.len(), 1);
    let indicator = terminal
        .pending_order_indicators
        .values()
        .next()
        .expect("next indicator");
    assert_eq!(indicator.kind, PendingOrderIndicatorKind::MarketPlacing);
    assert!(!indicator.is_buy);
    assert_eq!(indicator.size, "0.25");
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn market_click_does_not_wait_for_account_refresh_rate_limit_backoff() {
    let mut terminal = terminal_with_hud_chart(true);
    make_btc_tradeable(&mut terminal);
    let retry_due = TradingTerminal::now_ms().saturating_add(60_000);
    terminal.account_refresh_backoff_until_ms = Some(retry_due);
    let _retry = terminal.refresh_account_data();
    assert!(terminal.account_reconciliation_required);
    assert!(!terminal.account_loading);

    let _placement = terminal.handle_submit_hud_order(market_request());

    assert_eq!(
        order_status_of(&terminal),
        Some(("Placing HUD market LONG 1 BTC...", false))
    );
    assert_eq!(terminal.pending_order_indicators.len(), 1);
    assert!(error_toast_messages(&terminal).is_empty());
    assert_eq!(terminal.account_refresh_retry_due_ms, Some(retry_due));
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn uncertain_market_result_still_blocks_next_click_during_refresh() {
    for result in [
        Err("exchange request timed out".to_string()),
        Ok(order_response(serde_json::json!("waitingForFill"))),
    ] {
        let mut terminal = terminal_with_hud_chart(true);
        make_btc_tradeable(&mut terminal);
        let _placement = terminal.handle_submit_hud_order(market_request());
        let indicator_id = terminal.pending_order_indicators.keys().next().copied();
        let _reconciliation =
            terminal.handle_hud_order_result(indicator_id, None, market_context(), result);
        assert!(terminal.account_reconciliation_required);
        assert!(terminal.has_pending_one_shot_status_requests_for_test());

        let _placement = terminal.handle_submit_hud_order(market_request());

        assert_eq!(
            order_status_of(&terminal),
            Some((
                "Wait for pending trading requests to finish before placing a HUD order",
                true
            ))
        );
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());
    }
}

#[test]
fn market_click_during_refresh_still_requires_matching_account() {
    let mut terminal = terminal_with_hud_chart(true);
    make_btc_tradeable(&mut terminal);
    let _refresh = terminal.refresh_account_data();
    terminal.wallet_address_input = "0xdef0000000000000000000000000000000000000".to_string();

    let _placement = terminal.handle_submit_hud_order(market_request());

    assert_eq!(
        order_status_of(&terminal),
        Some((
            "Connected wallet no longer matches the active account; reconnect before trading",
            true
        ))
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn market_click_during_refresh_still_requires_fresh_price() {
    let mut terminal = terminal_with_hud_chart(true);
    make_btc_tradeable(&mut terminal);
    let _refresh = terminal.refresh_account_data();
    terminal.all_mids_updated_at_ms.insert("BTC".to_string(), 1);

    let _placement = terminal.handle_submit_hud_order(market_request());

    assert!(
        order_status_of(&terminal)
            .is_some_and(|(message, error)| error && message.starts_with("No mid price for BTC"))
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn market_click_during_refresh_still_rejects_incomplete_perp_state() {
    let mut terminal = terminal_with_hud_chart(true);
    make_btc_tradeable(&mut terminal);
    let mut data = AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: Default::default(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    };
    data.completeness.positions_actionable = false;
    data.completeness.positions_complete = false;
    terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    let _refresh = terminal.refresh_account_data();

    let _placement = terminal.handle_submit_hud_order(market_request());

    assert_eq!(
        order_status_of(&terminal),
        Some((
            "Perpetual account state is incomplete; refresh account data before placing an order",
            true
        ))
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}
