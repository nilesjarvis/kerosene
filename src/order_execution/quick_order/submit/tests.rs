use crate::account::{
    AccountData, AccountDataCompleteness, AccountDataSection, AssetPosition, ClearinghouseState,
    MarginSummary, Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::config::AccountProfile;
use crate::order_execution::{
    OneShotPlacementContext, OrderSurface, PendingOrderAction, QuickOrderForm, QuickOrderRecovery,
    QuickOrderSubmissionSnapshot,
};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::{ExchangeOrderKind, ExchangeResponse};
use crate::timeframe::Timeframe;

mod restoration;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 7,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn spot_symbol(key: &str, display_name: &str) -> ExchangeSymbol {
    let mut symbol = symbol(key, MarketType::Spot);
    symbol.display_name = Some(display_name.to_string());
    symbol
}

fn error_toast_messages(terminal: &TradingTerminal) -> Vec<&str> {
    terminal
        .toasts
        .iter()
        .filter(|toast| toast.is_error)
        .map(|toast| toast.message.as_str())
        .collect()
}

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "1.25".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 400.0,
        chart_h: 300.0,
    }
}

#[test]
fn quick_order_submission_snapshot_debug_redacts_symbol_and_form_values() {
    let snapshot = QuickOrderSubmissionSnapshot {
        surface_id: ChartSurfaceId::Docked(1),
        symbol_key: "SECRETCOIN".to_string(),
        form: QuickOrderForm {
            price: 98765.4321,
            quantity: "quantity-secret".to_string(),
            quantity_is_usd: true,
            percentage: 42.42,
            quantity_provenance: None,
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 400.0,
            chart_h: 300.0,
        },
        reduce_only: true,
        market_universe: Default::default(),
    };

    let rendered = format!("{snapshot:?}");

    assert!(rendered.contains("symbol_key: <redacted>"));
    assert!(rendered.contains("price: <redacted>"));
    assert!(rendered.contains("quantity: <redacted>"));
    assert!(rendered.contains("reduce_only: true"));
    for secret in ["SECRETCOIN", "quantity-secret", "98765.4321", "42.42"] {
        assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
    }
}

fn exchange_response(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": statuses
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

fn cancel_exchange_response(statuses: Vec<serde_json::Value>) -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "cancel",
            "data": {
                "statuses": statuses
            }
        }
    }))
    .expect("test cancel exchange response should deserialize")
}

fn malformed_ok_response() -> ExchangeResponse {
    serde_json::from_value(serde_json::json!({
        "status": "ok",
        "response": {
            "type": "order",
            "data": {
                "statuses": "schema-shifted"
            }
        }
    }))
    .expect("test exchange response should deserialize")
}

fn terminal_with_quick_order(chart_id: ChartId, chart_symbol: &str) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(TEST_ACCOUNT.to_string());
    terminal.wallet_address_input = TEST_ACCOUNT.to_string();
    terminal.accounts = vec![AccountProfile {
        secret_id: "acct-a".to_string(),
        name: "Account A".to_string(),
        wallet_address: TEST_ACCOUNT.to_string(),
        agent_key: sensitive_string("").into_zeroizing(),
        hydromancer_api_key: sensitive_string("").into_zeroizing(),
    }];
    terminal.active_account_index = 0;
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.exchange_symbols.clear();

    let mut instance = ChartInstance::new(chart_id, chart_symbol.to_string(), Timeframe::H1);
    instance.set_quick_order(quick_order_form());
    terminal.charts.insert(chart_id, instance);
    terminal
}

fn add_fresh_mid(terminal: &mut TradingTerminal, symbol: &str, mid: f64) {
    terminal.all_mids.insert(symbol.to_string(), mid);
    terminal
        .all_mids_updated_at_ms
        .insert(symbol.to_string(), TradingTerminal::now_ms());
}

fn account_data_with_margin(fetched_at_ms: u64, positions_complete: bool) -> AccountData {
    let mut completeness = AccountDataCompleteness::default();
    if !positions_complete {
        completeness.mark_incomplete(AccountDataSection::Positions, "positions unavailable");
    }

    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness,
        fetched_at_ms,
    }
}

fn account_data_with_position(fetched_at_ms: u64, coin: &str, szi: &str) -> AccountData {
    let mut data = account_data_with_margin(fetched_at_ms, true);
    data.clearinghouse.asset_positions = vec![AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
            entry_px: "100".to_string(),
            position_value: "0".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "0".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }];
    data
}

fn set_account_data_for_submit_test(
    terminal: &mut TradingTerminal,
    fetched_at_ms: u64,
    positions_complete: bool,
) {
    terminal.set_account_data_for_address_for_test(
        "0xabc0000000000000000000000000000000000000",
        account_data_with_margin(fetched_at_ms, positions_complete),
    );
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("missing order status"),
    }
}

fn chart_instance_or_panic(terminal: &TradingTerminal, chart_id: ChartId) -> &ChartInstance {
    match terminal.charts.get(&chart_id) {
        Some(instance) => instance,
        None => panic!("missing chart instance {chart_id}"),
    }
}

fn pending_one_shot_status_request() -> PendingOneShotStatusRequest {
    PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: "0xabc0000000000000000000000000000000000000".to_string(),
            cloid: "0x00000000000000000000000000000002".to_string(),
            surface: OrderSurface::QuickOrder,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Limit,
        },
    )
}

fn quick_order_or_panic(instance: &ChartInstance) -> &QuickOrderForm {
    match instance.quick_order.as_ref() {
        Some(form) => form,
        None => panic!("missing restored quick order"),
    }
}

fn quick_order_result_context(account: &str, symbol_key: &str) -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: account.to_string(),
        cloid: "0xquick".to_string(),
        surface: OrderSurface::QuickOrder,
        symbol_key: symbol_key.to_string(),
        order_kind: ExchangeOrderKind::Limit,
    }
}

fn valid_detached_surface(terminal: &mut TradingTerminal, chart_id: ChartId) -> ChartSurfaceId {
    let window_id = iced::window::Id::unique();
    let surface_id = ChartSurfaceId::Detached(window_id);
    terminal
        .detached_chart_windows
        .insert(window_id, DetachedChartWindowState::new(chart_id));
    terminal
        .charts
        .get_mut(&chart_id)
        .expect("chart should exist")
        .chart
        .set_surface_id(surface_id);
    surface_id
}

fn take_quick_order_for_recovery(
    terminal: &mut TradingTerminal,
    chart_id: ChartId,
    surface_id: Option<ChartSurfaceId>,
) -> QuickOrderRecovery {
    if let Some(surface_id) = surface_id {
        terminal
            .chart_quick_order_surface
            .insert(chart_id, surface_id);
    }
    let form = terminal
        .charts
        .get_mut(&chart_id)
        .and_then(|instance| instance.take_quick_order())
        .expect("quick order form should be active before simulated submit");
    terminal.chart_quick_order_surface.remove(&chart_id);
    QuickOrderRecovery {
        chart_id,
        form,
        surface_id,
    }
}

fn assert_quick_order_recovery_absent(terminal: &TradingTerminal, chart_id: ChartId) {
    let instance = chart_instance_or_panic(terminal, chart_id);
    assert!(instance.quick_order.is_none());
    assert!(!terminal.chart_quick_order_surface.contains_key(&chart_id));
}

fn assert_non_rejected_quick_order_result_does_not_restore(
    result: Result<ExchangeResponse, String>,
) {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = valid_detached_surface(&mut terminal, chart_id);
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));

    let _task = terminal.handle_quick_order_result(
        None,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        result,
    );

    assert_quick_order_recovery_absent(&terminal, chart_id);
}

#[test]
fn handle_submit_quick_order_sets_pending_order_action() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    add_fresh_mid(&mut terminal, "BTC", 100.0);

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
    assert_eq!(terminal.pending_order_indicators.len(), 1);
    assert!(
        terminal
            .charts
            .get(&chart_id)
            .and_then(|instance| instance.quick_order.as_ref())
            .is_none()
    );
}

#[test]
fn quick_order_placing_status_shows_spot_pair_name() {
    // Spot symbol keys are raw "@{index}" pair indices (HYPE/USDC is "@107");
    // the placement status must show the pair name instead.
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "@107");
    terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE/USDC")];
    add_fresh_mid(&mut terminal, "@107", 100.0);

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error, "unexpected error status: {message}");
    assert_eq!(message, "Placing limit BUY 1.25 HYPE/USDC...");
}

#[test]
fn quick_order_pending_gate_rejection_pushes_toast() {
    // Quick orders are submitted from charts, where the order ticket pane may
    // be closed; gate rejections must surface as a toast, not only in the
    // pane-local status line.
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.pending_order_action = Some(PendingOrderAction::Sell);

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    assert_eq!(
        error_toast_messages(&terminal),
        vec!["Wait for pending trading requests to finish before placing a quick order"]
    );
}

#[test]
fn quick_order_prepare_failure_pushes_toast() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    add_fresh_mid(&mut terminal, "BTC", 100.0);
    if let Some(form) = terminal
        .charts
        .get_mut(&chart_id)
        .and_then(|instance| instance.quick_order.as_mut())
    {
        form.quantity = "0".to_string();
    }

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    assert_eq!(
        error_toast_messages(&terminal),
        vec!["Invalid quantity for asset precision"]
    );
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
}

#[test]
fn stale_quick_order_snapshot_rejection_pushes_toast() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    add_fresh_mid(&mut terminal, "BTC", 100.0);
    let old_surface = ChartSurfaceId::Docked(chart_id);
    let old_form = quick_order_or_panic(chart_instance_or_panic(&terminal, chart_id));
    let snapshot = terminal.quick_order_submission_snapshot(chart_id, old_surface, old_form);
    let mut new_form = quick_order_form();
    new_form.quantity = "9.5".to_string();
    terminal
        .charts
        .get_mut(&chart_id)
        .expect("chart")
        .set_quick_order(new_form);

    let _task = terminal.handle_submit_quick_order_from_snapshot(chart_id, true, snapshot);

    assert_eq!(
        error_toast_messages(&terminal),
        vec!["Quick order changed; review and submit again"]
    );
}

#[test]
fn stale_quick_order_submit_message_does_not_clear_newer_surface_form() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    add_fresh_mid(&mut terminal, "BTC", 100.0);
    let old_surface = ChartSurfaceId::Docked(chart_id);
    let old_form = quick_order_or_panic(chart_instance_or_panic(&terminal, chart_id));
    let snapshot = terminal.quick_order_submission_snapshot(chart_id, old_surface, old_form);
    let new_surface = valid_detached_surface(&mut terminal, chart_id);
    let mut new_form = quick_order_form();
    new_form.quantity = "9.5".to_string();
    terminal
        .charts
        .get_mut(&chart_id)
        .expect("chart")
        .set_quick_order(new_form.clone());
    terminal
        .chart_quick_order_surface
        .insert(chart_id, new_surface);

    let _task = terminal.handle_submit_quick_order_from_snapshot(chart_id, true, snapshot);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "Quick order changed; review and submit again");
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert_eq!(instance.quick_order.as_ref(), Some(&new_form));
    assert_eq!(
        terminal.chart_quick_order_surface.get(&chart_id),
        Some(&new_surface)
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn handle_submit_quick_order_rejects_while_order_action_is_pending() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.pending_order_action = Some(PendingOrderAction::Sell);

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before placing a quick order"
    );
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Sell)
    );
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn handle_submit_quick_order_rejects_while_one_shot_status_pending() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.insert_pending_one_shot_status_request(pending_one_shot_status_request());

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before placing a quick order"
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn handle_submit_quick_order_rejects_while_account_reconciliation_is_pending() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.account_reconciliation_required = true;

    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Account refresh pending; wait for fresh account data before placing a quick order"
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
}

#[test]
fn handle_submit_quick_order_rejects_percentage_quantity_after_account_revision_change() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    set_account_data_for_submit_test(&mut terminal, TradingTerminal::now_ms(), true);
    add_fresh_mid(&mut terminal, "BTC", 100.0);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);
    terminal.bump_account_data_revision();
    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("older account snapshot"));
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn handle_submit_quick_order_rejects_percentage_quantity_from_stale_account_snapshot() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    let stale_fetched_at = TradingTerminal::now_ms()
        .saturating_sub(AccountData::POSITION_ACTION_MAX_AGE_MS)
        .saturating_sub(1);
    set_account_data_for_submit_test(&mut terminal, stale_fetched_at, true);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);
    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Account data is stale for percentage size"));
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn handle_submit_quick_order_rejects_percentage_quantity_from_incomplete_positions() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    set_account_data_for_submit_test(&mut terminal, TradingTerminal::now_ms(), false);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);
    let _task = terminal.handle_submit_quick_order(chart_id, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Positions may be incomplete"));
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn manual_quick_order_quantity_edit_clears_percentage_provenance_before_submit() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    set_account_data_for_submit_test(&mut terminal, TradingTerminal::now_ms(), true);
    add_fresh_mid(&mut terminal, "BTC", 100.0);

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);
    terminal.bump_account_data_revision();
    terminal.handle_quick_order_qty_changed(chart_id, "1.25".to_string());
    let instance = chart_instance_or_panic(&terminal, chart_id);
    let form = quick_order_or_panic(instance);
    assert!(form.quantity_provenance.is_none());
    let _task = terminal.handle_submit_quick_order(chart_id, true);

    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Buy),
        "status: {:?}",
        terminal.order_status
    );
    assert_eq!(terminal.pending_order_indicators.len(), 1);
    assert!(
        terminal
            .charts
            .get(&chart_id)
            .and_then(|instance| instance.quick_order.as_ref())
            .is_none()
    );
}

#[test]
fn handle_submit_quick_order_rejects_reduce_only_usd_percentage_after_reference_price_changes() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.order_reduce_only = true;
    terminal.set_account_data_for_address_for_test(
        "0xabc0000000000000000000000000000000000000",
        account_data_with_position(TradingTerminal::now_ms(), "BTC", "2"),
    );
    add_fresh_mid(&mut terminal, "BTC", 100.0);
    if let Some(form) = terminal
        .charts
        .get_mut(&chart_id)
        .and_then(|instance| instance.quick_order.as_mut())
    {
        form.quantity.clear();
        form.quantity_is_usd = true;
        form.is_limit = true;
        form.price = 100.0;
    }

    terminal.handle_quick_order_percentage_changed(chart_id, 25.0);
    if let Some(form) = terminal
        .charts
        .get_mut(&chart_id)
        .and_then(|instance| instance.quick_order.as_mut())
    {
        assert_eq!(form.quantity, "50.00");
        form.price = 50.0;
    }
    let _task = terminal.handle_submit_quick_order(chart_id, false);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("reference price changed"));
    let instance = chart_instance_or_panic(&terminal, chart_id);
    assert!(instance.quick_order.is_some());
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn handle_quick_order_result_clears_pending_order_action() {
    let mut terminal = TradingTerminal::boot().0;
    let account = "0xabc0000000000000000000000000000000000000";
    terminal.connected_address = Some(account.to_string());
    terminal.pending_order_action = Some(PendingOrderAction::Buy);
    let pending_id = terminal.add_pending_order_placement_indicator(
        account.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );
    let context = OneShotPlacementContext {
        account_address: account.to_string(),
        cloid: "0xquick".to_string(),
        surface: OrderSurface::QuickOrder,
        symbol_key: "BTC".to_string(),
        order_kind: ExchangeOrderKind::Limit,
    };

    let _task = terminal.handle_quick_order_result(
        pending_id,
        context,
        None,
        Err("network timeout".to_string()),
    );

    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn rejected_quick_order_result_restores_form_and_surface_mapping() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = valid_detached_surface(&mut terminal, chart_id);
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));
    terminal.pending_order_action = Some(PendingOrderAction::Buy);
    let pending_id = terminal.add_pending_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );

    let _task = terminal.handle_quick_order_result(
        pending_id,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.pending_order_indicators.is_empty());
    let instance = chart_instance_or_panic(&terminal, chart_id);
    let form = quick_order_or_panic(instance);
    assert!(instance.chart.quick_order_open);
    assert_eq!(form.quantity, "1.25");
    assert_eq!(
        terminal.chart_quick_order_surface.get(&chart_id),
        Some(&surface_id)
    );
    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Order rejected"));
}

#[test]
fn accepted_resting_quick_order_result_does_not_restore_form_or_surface_mapping() {
    assert_non_rejected_quick_order_result_does_not_restore(Ok(exchange_response(vec![
        serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        }),
    ])));
}

#[test]
fn filled_quick_order_result_does_not_restore_form_or_surface_mapping() {
    assert_non_rejected_quick_order_result_does_not_restore(Ok(exchange_response(vec![
        serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 43_u64
            }
        }),
    ])));
}

#[test]
fn ambiguous_quick_order_result_does_not_restore_form_or_surface_mapping() {
    assert_non_rejected_quick_order_result_does_not_restore(Ok(malformed_ok_response()));
}

#[test]
fn cancelled_quick_order_result_does_not_restore_form_or_surface_mapping() {
    assert_non_rejected_quick_order_result_does_not_restore(Ok(cancel_exchange_response(vec![
        serde_json::json!("success"),
    ])));
}

#[test]
fn transport_unknown_quick_order_result_does_not_restore_form_or_surface_mapping() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = valid_detached_surface(&mut terminal, chart_id);
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let _task = terminal.handle_quick_order_result(
        None,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        Err("network timeout".to_string()),
    );

    assert!(terminal.pending_order_action.is_none());
    assert_quick_order_recovery_absent(&terminal, chart_id);
    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("placement status unknown"));
}

#[test]
fn rejected_quick_order_result_after_account_switch_does_not_restore_recovery() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = valid_detached_surface(&mut terminal, chart_id);
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.handle_quick_order_result(
        None,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    assert_quick_order_recovery_absent(&terminal, chart_id);
}

#[test]
fn rejected_quick_order_result_does_not_restore_when_chart_symbol_changed() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = valid_detached_surface(&mut terminal, chart_id);
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));
    terminal
        .charts
        .get_mut(&chart_id)
        .expect("chart should exist")
        .symbol = "ETH".to_string();

    let _task = terminal.handle_quick_order_result(
        None,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    assert_quick_order_recovery_absent(&terminal, chart_id);
}

#[test]
fn rejected_quick_order_result_does_not_overwrite_active_quick_order() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = valid_detached_surface(&mut terminal, chart_id);
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));
    let mut newer_form = quick_order_form();
    newer_form.quantity = "9.5".to_string();
    terminal
        .charts
        .get_mut(&chart_id)
        .expect("chart should exist")
        .set_quick_order(newer_form);
    terminal
        .chart_quick_order_surface
        .insert(chart_id, surface_id);

    let _task = terminal.handle_quick_order_result(
        None,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    let instance = chart_instance_or_panic(&terminal, chart_id);
    let form = quick_order_or_panic(instance);
    assert_eq!(form.quantity, "9.5");
    assert_eq!(
        terminal.chart_quick_order_surface.get(&chart_id),
        Some(&surface_id)
    );
}

#[test]
fn rejected_quick_order_result_ignores_missing_or_closed_surface_mapping() {
    let chart_id = 42;
    let mut terminal = terminal_with_quick_order(chart_id, "BTC");
    let surface_id = ChartSurfaceId::Detached(iced::window::Id::unique());
    let recovery = take_quick_order_for_recovery(&mut terminal, chart_id, Some(surface_id));

    let _task = terminal.handle_quick_order_result(
        None,
        quick_order_result_context(TEST_ACCOUNT, "BTC"),
        Some(recovery),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    let instance = chart_instance_or_panic(&terminal, chart_id);
    let form = quick_order_or_panic(instance);
    assert_eq!(form.quantity, "1.25");
    assert!(!terminal.chart_quick_order_surface.contains_key(&chart_id));
}
