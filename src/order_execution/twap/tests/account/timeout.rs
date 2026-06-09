use super::{
    TwapStatus, empty_account_data, origin_account_terminal, reconciliation_twap, twap_by_id,
};

use std::time::Instant;

#[test]
fn reconciliation_timeout_fails_closed_when_account_fills_never_catch_up() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.account_data = Some(empty_account_data());
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
}
