use super::fixtures::{
    account_data_with_order, open_order, order_status_or_panic, outcome_symbol,
    terminal_with_move_order,
};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::order_execution::{
    MoveOrderKey, OneShotPlacementContext, OrderSurface, PendingLeverageUpdateContext,
    PendingMoveOrderContext, PendingNukeExecution, PendingOrderAction,
};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::ExchangeOrderKind;

const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

fn move_key(coin: &str) -> MoveOrderKey {
    MoveOrderKey::new(coin, 42)
}

fn one_shot_context() -> OneShotPlacementContext {
    OneShotPlacementContext {
        account_address: TEST_ACCOUNT.to_string(),
        cloid: "0xpending".to_string(),
        surface: OrderSurface::Ticket,
        symbol_key: "BTC".to_string(),
        order_kind: ExchangeOrderKind::Limit,
    }
}

fn pending_leverage_update() -> PendingLeverageUpdateContext {
    PendingLeverageUpdateContext {
        address: TEST_ACCOUNT.to_string(),
        symbol_key: "BTC".to_string(),
        display: "BTC".to_string(),
        asset: 0,
        dex: None,
        is_cross: true,
        leverage: 10,
    }
}

fn pending_move_context() -> PendingMoveOrderContext {
    PendingMoveOrderContext::new(
        TEST_ACCOUNT.to_string(),
        sensitive_string("move-agent").into_zeroizing(),
    )
    .expect("move context")
}

fn assert_move_waits_for_pending_trading_request(mut terminal: TradingTerminal) {
    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
    assert!(terminal.pending_order_indicators.is_empty());
    assert_eq!(
        terminal
            .order_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some((
            "Wait for pending trading requests to finish before moving orders",
            true
        ))
    );
}

#[test]
fn handle_move_order_blocks_far_away_drag_price() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 300.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("away from BTC reference 100"));
    assert!(message.contains("Press Mid or update the price"));
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_fails_closed_when_dragged_order_mid_is_missing() {
    let mut terminal = terminal_with_move_order("BTC", "ETH", 100.0);
    terminal.active_symbol = "ETH".to_string();

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("No mid price for BTC"));
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_allows_in_band_drag_price() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error);
    assert_eq!(message, "Moving BTC order to $101...");
    assert!(
        terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_uses_dragged_symbol_when_oid_collides() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .open_orders
        .push(open_order("ETH", 42, "200"));
    terminal.all_mids.insert("ETH".to_string(), 200.0);
    terminal
        .all_mids_updated_at_ms
        .insert("ETH".to_string(), TradingTerminal::now_ms());

    let _task = terminal.handle_move_order("ETH".to_string(), 42, 201.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error);
    assert_eq!(message, "Moving ETH order to $201...");
    assert!(
        terminal
            .pending_move_order_contexts
            .contains_key(&move_key("ETH"))
    );
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_refuses_pending_cancel_for_same_oid() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    let order = open_order("BTC", 42, "100");
    terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &order);

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("cancel already pending"));
    assert!(message.contains("order 42"));
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
    assert_eq!(terminal.pending_move_order_contexts.len(), 0);
}

#[test]
fn handle_move_order_waits_for_pending_order_action() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    assert_move_waits_for_pending_trading_request(terminal);
}

#[test]
fn handle_move_order_waits_for_pending_one_shot_status() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal.insert_pending_one_shot_status_request(PendingOneShotStatusRequest::new(
        7,
        &one_shot_context(),
    ));

    assert_move_waits_for_pending_trading_request(terminal);
}

#[test]
fn handle_move_order_waits_for_pending_leverage_update() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal.pending_leverage_update = Some(pending_leverage_update());

    assert_move_waits_for_pending_trading_request(terminal);
}

#[test]
fn handle_move_order_waits_for_pending_nuke_execution() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(7, 1, 0));

    assert_move_waits_for_pending_trading_request(terminal);
}

#[test]
fn handle_move_order_waits_for_other_pending_move_context() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal
        .pending_move_order_contexts
        .insert(MoveOrderKey::new("ETH", 43), pending_move_context());

    assert_move_waits_for_pending_trading_request(terminal);
}

#[test]
fn handle_move_order_refuses_while_account_refresh_is_loading() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal.account_loading = true;

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Account refresh in progress; wait for fresh open orders before moving"
    );
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_refuses_missing_open_order_snapshot() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal.account_data = None;

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "No account data available; refresh before moving");
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_refuses_stale_open_order_snapshot_and_refreshes() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .fetched_at_ms = 1;

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Open orders are stale"));
    assert!(message.contains("refresh before moving orders"));
    assert!(terminal.account_loading);
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_does_not_treat_positions_refresh_as_open_order_freshness() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    let now_ms = TradingTerminal::now_ms();
    let account_data = terminal.account_data.as_mut().expect("account data");
    account_data.fetched_at_ms = 1;
    account_data.mark_positions_fetched_at(now_ms);

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Open orders are stale"));
    assert!(message.contains("refresh before moving orders"));
    assert!(terminal.account_loading);
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_refuses_incomplete_open_orders_and_refreshes() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    terminal
        .account_data
        .as_mut()
        .expect("account data")
        .completeness
        .open_orders_complete = false;

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "Open orders are incomplete; refresh before moving");
    assert!(terminal.account_loading);
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_rejects_trigger_orders_before_building_modify_wire() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);
    let order = terminal
        .account_data
        .as_mut()
        .and_then(|data| data.open_orders.first_mut())
        .expect("fixture order");
    order.is_trigger = Some(true);
    order.trigger_px = Some("95".to_string());

    let _task = terminal.handle_move_order("BTC".to_string(), 42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("trigger orders"));
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("BTC"))
    );
}

#[test]
fn handle_move_order_rejects_non_tradable_fallback_outcome() {
    let mut terminal = terminal_with_move_order("#660", "#660", 0.42);
    terminal.exchange_symbols = vec![outcome_symbol("#660", true)];
    terminal.account_data_address = terminal.connected_address.clone();
    terminal.account_data = Some(account_data_with_order(open_order("#660", 42, "0.42")));

    let _task = terminal.handle_move_order("#660".to_string(), 42, 0.43);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("not a tradable market"));
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("#660"))
    );
}

#[test]
fn handle_move_order_status_uses_outcome_display_label() {
    let mut terminal = terminal_with_move_order("#670", "#670", 0.42);
    let symbol = outcome_symbol("#670", false);
    let label = TradingTerminal::exchange_symbol_display_name(&symbol);
    terminal.exchange_symbols = vec![symbol];
    let mut order = open_order("#670", 42, "0.42");
    order.sz = "2".to_string();
    terminal.account_data_address = terminal.connected_address.clone();
    terminal.account_data = Some(account_data_with_order(order));

    let _task = terminal.handle_move_order("#670".to_string(), 42, 0.43);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error);
    assert!(message.contains(&label));
    assert!(!message.contains("#670"));
}

#[test]
fn handle_move_order_rejects_fractional_outcome_size() {
    let mut terminal = terminal_with_move_order("#670", "#670", 0.42);
    terminal.exchange_symbols = vec![outcome_symbol("#670", false)];
    terminal.account_data_address = terminal.connected_address.clone();
    terminal.account_data = Some(account_data_with_order(open_order("#670", 42, "0.42")));

    let _task = terminal.handle_move_order("#670".to_string(), 42, 0.43);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("whole-contract sizes"));
    assert!(
        !terminal
            .pending_move_order_contexts
            .contains_key(&move_key("#670"))
    );
}
