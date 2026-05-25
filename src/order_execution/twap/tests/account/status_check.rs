use super::{
    CHILD_OID, CLOID, TWAP_RECONCILIATION_TIMEOUT, TwapChildStatus,
    disable_current_account_refresh, filled_status, origin_account_terminal,
    reconciliation_deadline, switched_account_terminal, test_twap, twap_by_id,
};

use std::time::Instant;

#[test]
fn filled_status_check_arms_reconciliation_deadline() {
    let now = Instant::now();
    let mut terminal = origin_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        Ok(filled_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    let deadline = reconciliation_deadline(twap);
    assert!(deadline > now);
    assert!(deadline <= Instant::now() + TWAP_RECONCILIATION_TIMEOUT);
}

#[test]
fn filled_status_check_after_account_switch_does_not_refresh_current_account() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    disable_current_account_refresh(&mut terminal);
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    let _task = terminal.handle_twap_order_status_result(
        1,
        CLOID.to_string(),
        Ok(filled_status(CLOID, CHILD_OID)),
    );

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_address, super::ORIGIN_ADDRESS);
    assert_eq!(
        twap.child_orders[0].status,
        TwapChildStatus::AwaitingReconciliation
    );
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert!(
        twap.reconciliation_deadline.is_some(),
        "filled status must wait for origin-account fills"
    );
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
}
