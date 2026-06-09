use super::*;
use crate::order_execution::PendingOrderAction;

#[test]
fn close_position_rejects_while_order_action_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.pending_order_action = Some(PendingOrderAction::ClosePosition);

    let _task = terminal.execute_close_position("BTC", 0.5, true);

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "Wait for the pending order action to finish");
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::ClosePosition)
    );
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
