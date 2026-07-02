use super::*;
use crate::order_execution::PendingOrderAction;

#[test]
fn close_position_rejects_while_order_action_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.pending_order_action = Some(PendingOrderAction::ClosePosition);

    let _task = terminal.execute_close_position("BTC", 0.5, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before closing positions"
    );
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::ClosePosition)
    );
}

#[test]
fn close_position_rejects_while_one_shot_status_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.insert_pending_one_shot_status_request(pending_one_shot_status_request());

    let _task = terminal.execute_close_position("BTC", 0.5, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before closing positions"
    );
    assert!(terminal.pending_order_action.is_none());
    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn close_position_refuses_missing_account_snapshot_without_pending_work() {
    let mut terminal = TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.account_data = None;
    terminal.account_loading = false;

    let _task = terminal.execute_close_position("BTC", 1.0, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "No account data available; refresh before closing");
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn close_position_refuses_stale_account_snapshot_and_requests_refresh() {
    let mut terminal = terminal_with_stale_account();

    let _task = terminal.execute_close_position("BTC", 1.0, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Account data is stale"));
    assert!(message.contains("refresh before closing positions"));
    assert!(terminal.account_loading);
}

#[test]
fn close_position_allows_degraded_fallback_snapshot() {
    // Regression: a complete Hyperliquid fallback snapshot (degraded, but with
    // usable positions) must not be blocked by the positions-incomplete gate,
    // and must not trigger the refresh loop that recreates the same flag.
    let mut terminal = terminal_with_degraded_fresh_account();

    let _task = terminal.execute_close_position("BTC", 1.0, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error, "expected a non-error status, got: {message}");
    assert!(message.contains("Closing"), "unexpected status: {message}");
    assert!(!message.contains("Positions may be incomplete"));
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::ClosePosition)
    );
    assert!(!terminal.account_loading);
}

#[test]
fn close_position_refuses_incomplete_position_snapshot_and_requests_refresh() {
    let mut terminal = terminal_with_incomplete_fresh_account();

    let _task = terminal.execute_close_position("BTC", 1.0, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Positions may be incomplete"));
    assert!(message.contains("HIP-3 positions unavailable"));
    assert!(message.contains("refresh before closing positions"));
    assert!(terminal.account_loading);
    assert_eq!(terminal.pending_order_action, None);
    assert!(terminal.pending_order_indicators.is_empty());
}
