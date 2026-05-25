use super::{
    ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason, chase_order, chase_order_by_id,
    connected_terminal_with_chase_account, fill_with_oid, open_order, terminal_with_chase_fills,
};

#[test]
fn chase_fill_reconciliation_removes_fully_filled_chase() {
    let mut terminal =
        terminal_with_chase_fills(chase_order(), vec![fill_with_oid(1_001, 42, "100", "1.0")]);

    let _task = terminal.reconcile_chase_after_account_refresh();

    assert!(terminal.chase_orders.is_empty());
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| !*is_error && message.contains("Chase filled"))
    );
}

#[test]
fn live_fill_reconciliation_waits_for_fresh_open_orders_before_removal() {
    let mut terminal =
        terminal_with_chase_fills(chase_order(), vec![fill_with_oid(1_001, 42, "100", "1.0")]);

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
}

#[test]
fn completed_chase_cancels_live_known_order_before_removal() {
    let mut terminal = connected_terminal_with_chase_account(
        chase_order(),
        vec![fill_with_oid(1_001, 42, "100", "1.0")],
        vec![open_order(42, Some(false))],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}

#[test]
fn overfilled_chase_preserves_raw_total_and_cancels_live_known_order() {
    let mut chase = chase_order();
    chase.known_oids.push(43);
    let mut terminal = connected_terminal_with_chase_account(
        chase,
        vec![fill_with_oid(1_001, 42, "100", "1.2")],
        vec![open_order(43, Some(false))],
    );

    let _task = terminal.reconcile_chase_fills_from_account();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.filled_size, 1.2);
    assert_eq!(chase.remaining_size, 0.0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 43 }
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| { *is_error && message.contains("over target") })
    );
}
