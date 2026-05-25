use super::{
    CHILD_OID, CLOID, ORIGIN_ADDRESS, SWITCHED_ADDRESS, TwapChildStatus,
    disable_current_account_refresh, empty_account_data, pending_twap, reconciliation_twap,
    switched_account_terminal, test_twap, twap_by_id, user_fill,
};

use std::time::Instant;

#[test]
fn twap_status_checks_resolve_origin_account_after_account_switch() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, test_twap(1, CLOID, now));

    assert_eq!(
        terminal.twap_origin_address(1).as_deref(),
        Some(ORIGIN_ADDRESS)
    );
}

#[test]
fn twap_reconciliation_uses_fetched_account_scope_after_account_switch() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let fills = vec![user_fill(CHILD_OID, "0.5", "100")];

    terminal.reconcile_twap_fills_for_account(SWITCHED_ADDRESS, &fills);
    assert_eq!(
        terminal.twap_orders.get(&1).map(|twap| twap.filled_size),
        Some(0.0)
    );

    terminal.reconcile_twap_fills_for_account(ORIGIN_ADDRESS, &fills);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
    assert_eq!(twap.status_check_cloid, None);
    assert_eq!(twap.reconciliation_deadline, None);
}

#[test]
fn stale_account_data_loaded_reconciles_twap_without_replacing_current_account() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    terminal.account_data = Some(empty_account_data());
    terminal.twap_orders.insert(1, reconciliation_twap(now));
    let mut stale_data = empty_account_data();
    stale_data.fills.push(user_fill(CHILD_OID, "0.5", "100"));

    let _task = terminal.apply_account_data_loaded(ORIGIN_ADDRESS.to_string(), Ok(stale_data));

    assert_eq!(
        terminal.connected_address.as_deref(),
        Some(SWITCHED_ADDRESS)
    );
    assert_eq!(
        terminal.account_data.as_ref().map(|data| data.fills.len()),
        Some(0)
    );
    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.filled_size, 0.5);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Filled);
}

#[test]
fn ambiguous_slice_result_after_account_switch_does_not_refresh_current_account() {
    let now = Instant::now();
    let mut terminal = switched_account_terminal();
    disable_current_account_refresh(&mut terminal);
    terminal.twap_orders.insert(1, pending_twap(1, CLOID, now));

    let _task =
        terminal.handle_twap_slice_result(1, Err("Exchange request failed after submit".into()));

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(twap.account_address, ORIGIN_ADDRESS);
    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
    assert!(!terminal.account_loading);
    assert!(!terminal.account_reconciliation_required);
}
