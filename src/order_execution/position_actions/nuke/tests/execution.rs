use super::{order_status_or_panic, terminal_with_stale_account};

#[test]
fn execute_nuke_refuses_stale_account_snapshot_and_requests_refresh() {
    let mut terminal = terminal_with_stale_account();

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Account data is stale"));
    assert!(message.contains("refresh before NUKE"));
    assert!(terminal.account_loading);
}
