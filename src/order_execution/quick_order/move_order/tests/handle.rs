use super::fixtures::{
    account_data_with_order, open_order, order_status_or_panic, outcome_symbol,
    terminal_with_move_order,
};
use crate::app_state::TradingTerminal;

#[test]
fn handle_move_order_blocks_far_away_drag_price() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);

    let _task = terminal.handle_move_order(42, 300.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("away from BTC reference 100"));
    assert!(message.contains("Press Mid or update the price"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn handle_move_order_fails_closed_when_dragged_order_mid_is_missing() {
    let mut terminal = terminal_with_move_order("BTC", "ETH", 100.0);
    terminal.active_symbol = "ETH".to_string();

    let _task = terminal.handle_move_order(42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("No mid price for BTC"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn handle_move_order_allows_in_band_drag_price() {
    let mut terminal = terminal_with_move_order("BTC", "BTC", 100.0);

    let _task = terminal.handle_move_order(42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error);
    assert_eq!(message, "Moving BTC order to $101...");
    assert!(terminal.pending_move_order_contexts.contains_key(&42));
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

    let _task = terminal.handle_move_order(42, 101.0);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("trigger orders"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn handle_move_order_rejects_non_tradable_fallback_outcome() {
    let mut terminal = terminal_with_move_order("#660", "#660", 0.42);
    terminal.exchange_symbols = vec![outcome_symbol("#660", true)];
    terminal.account_data = Some(account_data_with_order(open_order("#660", 42, "0.42")));

    let _task = terminal.handle_move_order(42, 0.43);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("not a tradable market"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}

#[test]
fn handle_move_order_status_uses_outcome_display_label() {
    let mut terminal = terminal_with_move_order("#670", "#670", 0.42);
    let symbol = outcome_symbol("#670", false);
    let label = TradingTerminal::exchange_symbol_display_name(&symbol);
    terminal.exchange_symbols = vec![symbol];
    let mut order = open_order("#670", 42, "0.42");
    order.sz = "2".to_string();
    terminal.account_data = Some(account_data_with_order(order));

    let _task = terminal.handle_move_order(42, 0.43);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error);
    assert!(message.contains(&label));
    assert!(!message.contains("#670"));
}

#[test]
fn handle_move_order_rejects_fractional_outcome_size() {
    let mut terminal = terminal_with_move_order("#670", "#670", 0.42);
    terminal.exchange_symbols = vec![outcome_symbol("#670", false)];
    terminal.account_data = Some(account_data_with_order(open_order("#670", 42, "0.42")));

    let _task = terminal.handle_move_order(42, 0.43);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("whole-contract sizes"));
    assert!(!terminal.pending_move_order_contexts.contains_key(&42));
}
