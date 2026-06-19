use super::*;

#[test]
fn chase_oid_status_error_keeps_chase_uncertain_for_reconciliation() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Modify,
    };
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_oid_status_result(
        1,
        9001,
        Err("status endpoint down: token=super-secret".into()),
    );

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
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("token=<redacted>"));
    assert!(!message.contains("super-secret"));
}

#[test]
fn chase_oid_status_ignores_mismatched_returned_oid() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Modify,
    };
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_oid_status_result(
        1,
        9001,
        Ok(OrderStatusResult {
            status: "open".to_string(),
            oid: Some(9002),
            cloid: None,
            raw_summary: "open (oid 9002)".to_string(),
        }),
    );

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(chase.current_oid, Some(9001));
    assert_eq!(chase.known_oids, Vec::<u64>::new());
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify
        }
    );
    assert!(order_status_is_error_containing(
        &terminal,
        "response oid did not match"
    ));
}

#[test]
fn chase_oid_status_open_does_not_duplicate_in_flight_stop_cancel() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::Canceling { oid: 9001 },
    };
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_oid_status_result(1, 9001, Ok(open_order_status()));

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 9001 }
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                !*is_error && message.contains("cancel already in flight")
            })
    );
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
fn stopping_chase_oid_status_terminal_archives_when_disconnected() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 9001 },
    };
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_oid_status_result(1, 9001, Ok(oid_status("canceled")));

    assert!(terminal.chase_orders.is_empty());
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("order no longer open")
            })
    );
    assert_eq!(terminal.advanced_order_history.len(), 1);
}

#[test]
fn disconnected_stopping_chase_oid_missing_status_archives_without_refreshing_current_account() {
    let mut chase = chase();
    chase.current_oid = Some(9001);
    chase.account_address = "0xabc0000000000000000000000000000000000000".to_string();
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 9001 },
    };
    let mut terminal = terminal_with_chase(chase);
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());

    let _task =
        terminal.handle_chase_order_oid_status_result(1, 9001, Ok(oid_status("unknownOid")));

    assert!(!terminal.account_loading);
    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.advanced_order_history.len(), 1);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("did not find previous account order 9001")
            })
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
