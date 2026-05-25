use super::*;

mod chase_fills;
mod chase_reprice;
mod chase_stop;
mod fills;
mod fixtures;
mod orders;

use fixtures::{account_data_with_timestamp, open_order};

#[test]
fn lagged_connected_user_stream_marks_account_loading_immediately() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;

    let _task = terminal.apply_ws_user_data_update(
        terminal.connected_address.clone(),
        WsUserData::Lagged { skipped: 3 },
    );

    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    assert_eq!(terminal.account_error, None);
}

#[test]
fn non_position_ws_updates_do_not_refresh_position_snapshot_timestamp() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![open_order(42, Some(false))],
        },
    );

    assert_eq!(
        terminal
            .account_data
            .as_ref()
            .map(|data| data.fetched_at_ms),
        Some(1_000)
    );
}

#[test]
fn lagged_non_connected_user_stream_does_not_mark_main_account_loading() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;

    let _task = terminal.apply_ws_user_data_update(
        Some("0xdef0000000000000000000000000000000000000".to_string()),
        WsUserData::Lagged { skipped: 3 },
    );

    assert!(!terminal.account_loading);
}

#[test]
fn websocket_account_repair_skips_when_initial_fetch_is_loading() {
    assert!(!should_repair_account_from_ws(
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        false,
        true,
    ));
    assert!(should_repair_account_from_ws(
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        false,
        false,
    ));
    assert!(!should_repair_account_from_ws(
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        true,
        false,
    ));
    assert!(!should_repair_account_from_ws(None, false, false));
}
