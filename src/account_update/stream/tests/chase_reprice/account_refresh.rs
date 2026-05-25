use super::{
    ChaseLifecycle, ChaseVerificationReason, account_data_with_timestamp, chase_order_by_id,
    open_order, refresh_ready_terminal, reprice_verification_chase, verifying_chase,
};

#[test]
fn chase_reprice_reconciliation_pauses_on_incomplete_account_snapshot() {
    let mut terminal = refresh_ready_terminal();
    let chase = verifying_chase(ChaseVerificationReason::Reprice);
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    data.completeness.fills_complete = false;
    terminal.account_data = Some(data);

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(chase.current_oid, Some(42));
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error && message.contains("Chase paused"))
    );
}

#[test]
fn chase_reprice_reconciliation_clears_confirmed_pending_target() {
    let mut terminal = refresh_ready_terminal();
    terminal
        .chase_orders
        .insert(1, reprice_verification_chase());
    let mut data = account_data_with_timestamp(1_000);
    let mut order = open_order(42, Some(false));
    order.limit_px = "101".to_string();
    data.open_orders = vec![order];
    terminal.account_data = Some(data);

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.desired_price, None);
}

#[test]
fn placement_without_oid_does_not_place_replacement_after_refresh() {
    let mut terminal = refresh_ready_terminal();
    let mut chase = verifying_chase(ChaseVerificationReason::Placement);
    chase.current_oid = None;
    terminal.chase_orders.insert(1, chase);
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, None);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
}

#[test]
fn missing_current_order_checks_status_before_replacement() {
    let mut terminal = refresh_ready_terminal();
    terminal
        .chase_orders
        .insert(1, verifying_chase(ChaseVerificationReason::MissingOrder));
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, Some(42));
    assert_eq!(chase.place_attempt_count, 0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, _is_error)| message.contains("checking order status"))
    );
}

#[test]
fn no_fill_terminal_status_allows_clean_replacement() {
    let mut terminal = refresh_ready_terminal();
    terminal.chase_orders.insert(
        1,
        verifying_chase(ChaseVerificationReason::MissingOrderResolvedNoFill),
    );
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, None);
    assert_eq!(chase.place_attempt_count, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
}
