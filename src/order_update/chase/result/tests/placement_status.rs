use super::*;

#[test]
fn chase_place_status_open_recovers_oid_after_unknown_place_response() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Placing;
    chase.current_cloid = Some(TEST_CLOID.to_string());
    let mut terminal = connected_terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Ok(open_order_status()),
    );

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(chase.current_oid, Some(9001));
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
    assert!(chase.known_oids.contains(&9001));
}

#[test]
fn chase_place_status_ignores_mismatched_returned_cloid() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Placement,
    };
    chase.current_cloid = Some(TEST_CLOID.to_string());
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Ok(OrderStatusResult {
            status: "open".to_string(),
            oid: Some(9001),
            cloid: Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            raw_summary: "open (oid 9001, cloid 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)".to_string(),
        }),
    );

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(chase.current_oid, None);
    assert_eq!(chase.known_oids, Vec::<u64>::new());
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
    assert!(order_status_is_error_containing(
        &terminal,
        "response cloid did not match"
    ));
}

#[test]
fn chase_place_status_error_keeps_chase_uncertain_for_retry() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Placement,
    };
    chase.current_cloid = Some(TEST_CLOID.to_string());
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Err("status endpoint down: api_key=super-secret".to_string()),
    );

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement
        }
    );
    assert!(order_status_is_error_containing(
        &terminal,
        "placement status still uncertain"
    ));
    let (message, is_error) = terminal.order_status.as_ref().expect("order status");
    assert!(*is_error);
    assert!(message.contains("api_key=<redacted>"));
    assert!(!message.contains("super-secret"));
}

#[test]
fn chase_place_status_rejected_without_oid_removes_chase() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Placement,
    };
    chase.current_cloid = Some(TEST_CLOID.to_string());
    chase.desired_price = Some(101.0);
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Ok(OrderStatusResult {
            status: "rejected".to_string(),
            oid: None,
            cloid: Some(TEST_CLOID.to_string()),
            raw_summary: "rejected (cloid 0x1234567890abcdef1234567890abcdef)".to_string(),
        }),
    );

    assert!(terminal.chase_orders.is_empty());
    assert!(order_status_is_error_containing(
        &terminal,
        "placement resolved without fill"
    ));
}

#[test]
fn chase_place_missing_status_preserves_stopping_state() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingPlace,
    };
    chase.current_cloid = Some(TEST_CLOID.to_string());
    chase.desired_price = Some(101.0);
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Ok(OrderStatusResult {
            status: "unknownOid".to_string(),
            oid: None,
            cloid: Some(TEST_CLOID.to_string()),
            raw_summary: "unknownOid (cloid 0x1234567890abcdef1234567890abcdef)".to_string(),
        }),
    );

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingPlace
        }
    );
}

#[test]
fn disconnected_stopping_chase_place_missing_status_does_not_refresh_current_account() {
    let mut chase = chase();
    chase.account_address = "0xabc0000000000000000000000000000000000000".to_string();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingPlace,
    };
    chase.current_cloid = Some(TEST_CLOID.to_string());
    chase.desired_price = Some(101.0);
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    let mut terminal = terminal_with_chase(chase);
    terminal.connected_address = Some("0xdef0000000000000000000000000000000000000".to_string());

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Ok(OrderStatusResult {
            status: "unknownOid".to_string(),
            oid: None,
            cloid: Some(TEST_CLOID.to_string()),
            raw_summary: "unknownOid (cloid 0x1234567890abcdef1234567890abcdef)".to_string(),
        }),
    );

    assert!(!terminal.account_loading);
    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingPlace
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("placement status ambiguous")
            })
    );
}

#[test]
fn chase_place_no_fill_status_preserves_stopping_state() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingPlace,
    };
    chase.current_cloid = Some(TEST_CLOID.to_string());
    chase.desired_price = Some(101.0);
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    let mut terminal = terminal_with_chase(chase);

    let _task = terminal.handle_chase_order_status_result(
        1,
        TEST_CLOID.to_string(),
        Ok(OrderStatusResult {
            status: "canceled".to_string(),
            oid: None,
            cloid: Some(TEST_CLOID.to_string()),
            raw_summary: "canceled (cloid 0x1234567890abcdef1234567890abcdef)".to_string(),
        }),
    );

    let chase = chase_from_terminal(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingPlace
        }
    );
    assert_eq!(chase.desired_price, None);
    assert_eq!(
        chase.stop_reason,
        Some(("Chase stopped".to_string(), false))
    );
}
