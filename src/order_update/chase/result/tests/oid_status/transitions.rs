use super::*;
use crate::signing::ChaseQueuedAction;

fn connections() -> [Option<String>; 4] {
    [
        None,
        Some("0xdef0000000000000000000000000000000000000".to_string()),
        Some(TEST_ACCOUNT.to_string()),
        Some(format!("  {}  ", TEST_ACCOUNT.to_ascii_uppercase())),
    ]
}

#[test]
fn chase_status_requests_and_errors_preserve_verification_intent() {
    let modify = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Modify,
    };
    let missing = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    let stopping = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 9001 },
    };
    let cases = [
        (ChaseLifecycle::LoadingBook, modify),
        (ChaseLifecycle::Placing, modify),
        (ChaseLifecycle::Resting, modify),
        (
            ChaseLifecycle::Queued {
                action: ChaseQueuedAction::Place,
            },
            modify,
        ),
        (
            ChaseLifecycle::Queued {
                action: ChaseQueuedAction::Reprice,
            },
            modify,
        ),
        (
            ChaseLifecycle::Queued {
                action: ChaseQueuedAction::SizeCorrection,
            },
            modify,
        ),
        (ChaseLifecycle::Modifying { oid: 42 }, modify),
        (
            ChaseLifecycle::Verifying {
                reason: ChaseVerificationReason::Placement,
            },
            modify,
        ),
        (
            ChaseLifecycle::Verifying {
                reason: ChaseVerificationReason::Reprice,
            },
            modify,
        ),
        (
            ChaseLifecycle::Verifying {
                reason: ChaseVerificationReason::SizeCorrection,
            },
            modify,
        ),
        (modify, modify),
        (missing, missing),
        (
            ChaseLifecycle::Verifying {
                reason: ChaseVerificationReason::MissingOrderResolvedNoFill,
            },
            missing,
        ),
        (
            ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingPlace,
            },
            stopping,
        ),
        (
            ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingModify { oid: 42 },
            },
            stopping,
        ),
        (
            ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::Canceling { oid: 42 },
            },
            stopping,
        ),
        (
            ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
            },
            stopping,
        ),
    ];

    for (initial, expected) in cases {
        for start_request in [false, true] {
            for (connection, connected_address) in connections().into_iter().enumerate() {
                let mut chase = chase();
                chase.current_oid = Some(9001);
                chase.known_oids = vec![42];
                chase.lifecycle = initial;
                chase.filled_size = 0.25;
                chase.remaining_size = 0.75;
                chase.desired_price = Some(101.0);
                chase.cancel_retries = 2;
                chase.stop_reason = Some(("existing reason".to_string(), true));
                let mut terminal = terminal_with_chase(chase);
                terminal.connected_address = connected_address;
                terminal.account_loading = false;
                terminal.pending_order_action = Some(PendingOrderAction::Buy);

                let before = Instant::now();
                let task = if start_request {
                    terminal.check_chase_order_status(1, 9001, "checking: token=fixture-secret")
                } else {
                    terminal.handle_chase_order_oid_status_result(
                        1,
                        9001,
                        Err("checking: token=fixture-secret".to_string()),
                    )
                };

                let chase = chase_from_terminal(&terminal, 1);
                assert_eq!(
                    chase.lifecycle, expected,
                    "{initial:?}, request: {start_request}"
                );
                assert_eq!(chase.current_oid, Some(9001));
                assert_eq!(chase.known_oids, vec![42]);
                assert_eq!(chase.filled_size, 0.25);
                assert_eq!(chase.remaining_size, 0.75);
                assert_eq!(chase.desired_price, Some(101.0));
                assert_eq!(chase.cancel_retries, 2);
                assert_eq!(
                    chase.stop_reason,
                    Some(("existing reason".to_string(), true))
                );
                assert!(
                    chase
                        .last_reprice_at
                        .is_some_and(|at| at >= before && at <= Instant::now())
                );
                let refresh = connection >= 2;
                assert_eq!(terminal.account_loading, refresh);
                assert_eq!(task.units() > 0, start_request || refresh);
                assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
                let (status, is_error) =
                    terminal.order_status.as_ref().expect("verification status");
                assert_eq!(*is_error, !start_request || !refresh);
                assert!(status.contains("token=<redacted>"));
                assert!(!status.contains("fixture-secret"));
            }
        }
    }
}

#[test]
fn chase_oid_outcomes_preserve_stop_and_account_reconciliation_rules() {
    for status in ["filled", "rejected", "canceled", "unknownOid"] {
        for stopping in [false, true] {
            for (connection, connected_address) in connections().into_iter().enumerate() {
                let mut chase = chase();
                chase.current_oid = Some(9001);
                chase.lifecycle = if stopping {
                    ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { oid: 9001 },
                    }
                } else {
                    ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    }
                };
                chase.filled_size = 0.25;
                chase.remaining_size = 0.75;
                chase.desired_price = Some(101.0);
                chase.cancel_retries = 2;
                chase.stop_reason = stopping.then(|| ("user stopped".to_string(), false));
                let mut terminal = terminal_with_chase(chase);
                terminal.connected_address = connected_address;
                terminal.account_loading = false;

                let task =
                    terminal.handle_chase_order_oid_status_result(1, 9001, Ok(oid_status(status)));

                let connected_to_owner = connection >= 2;
                if stopping && !connected_to_owner {
                    assert!(terminal.chase_orders.is_empty(), "{status}");
                    assert_eq!(terminal.advanced_order_history.len(), 1);
                    let entry = &terminal.advanced_order_history[0];
                    assert_eq!(entry.source_id, 1);
                    assert_eq!(entry.filled_size, 0.25);
                    assert_eq!(entry.remaining_size, 0.75);
                    assert!(!terminal.account_loading);
                    assert_eq!(task.units(), 0);
                    continue;
                }

                let chase = chase_from_terminal(&terminal, 1);
                let expected_lifecycle = match status {
                    "filled" => ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    },
                    "rejected" if !stopping => ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrderResolvedNoFill,
                    },
                    "unknownOid" if !stopping => ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    },
                    _ => ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { oid: 9001 },
                    },
                };
                assert_eq!(chase.lifecycle, expected_lifecycle, "{status}");
                assert_eq!(chase.current_oid, Some(9001));
                assert_eq!(
                    chase.filled_size,
                    if status == "filled" { 1.0 } else { 0.25 }
                );
                assert_eq!(
                    chase.remaining_size,
                    if status == "filled" { 0.0 } else { 0.75 }
                );
                assert_eq!(chase.cancel_retries, 2);
                assert_eq!(
                    chase.desired_price,
                    if status == "canceled" {
                        None
                    } else {
                        Some(101.0)
                    }
                );
                if status == "canceled" {
                    assert!(
                        chase
                            .stop_reason
                            .as_ref()
                            .is_some_and(|(reason, is_error)| *is_error
                                && reason.contains("no replacement will be placed"))
                    );
                } else {
                    assert_eq!(
                        chase.stop_reason,
                        stopping.then(|| ("user stopped".to_string(), false))
                    );
                }
                assert!(terminal.advanced_order_history.is_empty());
                let refresh = connected_to_owner && (stopping || status != "unknownOid");
                assert_eq!(terminal.account_loading, refresh, "{status}");
                assert_eq!(task.units() > 0, refresh, "{status}");
            }
        }
    }
}
