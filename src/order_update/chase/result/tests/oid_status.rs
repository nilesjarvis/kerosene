use super::*;

#[test]
fn chase_oid_status_error_keeps_chase_uncertain_for_reconciliation() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Modify,
    };
    let mut terminal = terminal_with_chase(chase);

    let _task =
        terminal.handle_chase_order_oid_status_result(1, 9001, Err("status endpoint down".into()));

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert!(order_status_is_error_containing(
        &terminal,
        "order status still uncertain"
    ));
}

#[test]
fn chase_oid_status_canceled_stops_without_authorizing_replacement() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.desired_price = Some(101.0);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_oid_status_result(1, 9001, Ok(oid_status("canceled")));

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(chase.desired_price, None);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid: 9001 }
        }
    );
}

#[test]
fn chase_oid_status_rejected_authorizes_replacement_after_refresh() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.desired_price = Some(101.0);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_oid_status_result(1, 9001, Ok(oid_status("rejected")));

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrderResolvedNoFill
        }
    );
}
