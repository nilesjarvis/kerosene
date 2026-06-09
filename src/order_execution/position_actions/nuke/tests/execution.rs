use super::{
    active_position, exchange_symbol, order_status_or_panic, stale_account_data,
    terminal_with_stale_account,
};

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

#[test]
fn execute_nuke_aborts_loudly_when_hidden_exposure_cannot_route() {
    let mut terminal = terminal_with_stale_account();
    terminal.exchange_symbols = vec![exchange_symbol("HIDDEN")];
    terminal.muted_tickers.insert("HIDDEN".to_string());
    let mut data = stale_account_data();
    data.fetched_at_ms = crate::app_time::now_ms();
    data.clearinghouse.asset_positions = vec![active_position("HIDDEN", "1")];
    terminal.account_data = Some(data);

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "NUKE aborted: hidden exposure could not be routed. Hidden skipped: HIDDEN (no mid price)"
    );
}
