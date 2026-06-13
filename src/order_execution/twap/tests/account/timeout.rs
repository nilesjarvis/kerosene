use super::{
    CLOID, TwapStatus, empty_account_data, origin_account_terminal, reconciliation_twap,
    set_account_data_for_connected_account, twap_by_id,
};

use std::time::Instant;

#[test]
fn reconciliation_timeout_fails_closed_when_account_fills_never_catch_up() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    set_account_data_for_connected_account(&mut terminal, empty_account_data());
    let mut twap = reconciliation_twap(now);
    twap.reconciliation_deadline = Some(now);
    terminal.twap_orders.insert(1, twap);

    terminal.reconcile_twap_fills_from_account();

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Error);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
    assert!(
        twap.events
            .iter()
            .any(|event| event.is_error && event.message.contains("Could not reconcile slice")),
        "timeout should leave an actionable error event"
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("Could not reconcile slice")
            }),
        "timeout should surface through order status"
    );
}

#[test]
fn reconciliation_timeout_names_unresolved_child_when_status_cloid_is_missing() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    set_account_data_for_connected_account(&mut terminal, empty_account_data());
    let mut twap = reconciliation_twap(now);
    twap.status_check_cloid = None;
    twap.reconciliation_deadline = Some(now);
    terminal.twap_orders.insert(1, twap);

    terminal.reconcile_twap_fills_from_account();

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Error);
    let expected_message = format!("Could not reconcile slice {CLOID}");
    assert!(
        twap.events
            .iter()
            .any(|event| { event.is_error && event.message.contains(&expected_message) }),
        "timeout should identify the unresolved child even when status_check_cloid is absent"
    );
}

#[test]
fn reconciliation_from_account_ignores_blank_connected_address() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.connected_address = Some("   ".to_string());
    terminal.account_data = Some(empty_account_data());
    let mut twap = reconciliation_twap(now);
    twap.reconciliation_deadline = Some(now);
    terminal.twap_orders.insert(1, twap);

    terminal.reconcile_twap_fills_from_account();

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.reconciliation_deadline, Some(now));
}

#[test]
fn reconciliation_from_account_ignores_mismatched_snapshot_owner() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.account_data_address = Some("0xdef".to_string());
    terminal.account_data = Some(empty_account_data());
    let mut twap = reconciliation_twap(now);
    twap.reconciliation_deadline = Some(now);
    terminal.twap_orders.insert(1, twap);

    terminal.reconcile_twap_fills_from_account();

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Paused);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.reconciliation_deadline, Some(now));
}

#[test]
fn twap_tick_expires_reconciliation_timeout_without_new_account_fills() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    let mut twap = reconciliation_twap(now);
    twap.reconciliation_deadline = Some(now);
    terminal.twap_orders.insert(1, twap);

    let _task = terminal.handle_twap_tick();

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status, TwapStatus::Error);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("Could not reconcile slice")
            }),
        "timer-driven timeout should surface through order status"
    );
}
