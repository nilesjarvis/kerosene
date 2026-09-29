use super::*;

const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

#[test]
fn stale_chase_results_preserve_state_and_keep_handler_specific_refresh_policy() {
    for handler in ["place", "modify", "cancel"] {
        for has_chase in [false, true] {
            for connected_to_owner in [false, true] {
                for outcome in ["resting", "rejected", "transport"] {
                    let mut chase = chase();
                    chase.lifecycle = ChaseLifecycle::Resting;
                    chase.current_cloid = Some(TEST_CLOID.to_string());
                    chase.place_attempt_count = 1;
                    chase.desired_price = Some(101.0);
                    chase.cancel_retries = 2;
                    let mut terminal = terminal_with_chase(chase);
                    if !has_chase {
                        terminal.chase_orders.clear();
                    }
                    terminal.connected_address = Some(
                        if connected_to_owner {
                            TEST_ACCOUNT
                        } else {
                            OTHER_ACCOUNT
                        }
                        .to_string(),
                    );
                    terminal.pending_order_action = Some(PendingOrderAction::ChaseBuy);
                    terminal.order_status = Some(("existing status".to_string(), false));
                    terminal.account_loading = false;
                    let result = match outcome {
                        "resting" => Ok(exchange_response(vec![serde_json::json!({
                            "resting": {"oid": 9001}
                        })])),
                        "rejected" => Ok(exchange_response(vec![serde_json::json!({
                            "error": "tick rejected"
                        })])),
                        _ => Err("connection closed".to_string()),
                    };

                    let task = match handler {
                        "place" => terminal.handle_chase_place_result(1, result),
                        "modify" => terminal.handle_chase_modify_result(1, 9001, result),
                        _ => terminal.handle_chase_cancel_result(1, 9001, result),
                    };

                    let refresh = has_chase
                        && connected_to_owner
                        && handler != "modify"
                        && outcome != "rejected";
                    assert_eq!(terminal.account_loading, refresh, "{handler}: {outcome}");
                    assert_eq!(task.units() > 0, refresh, "{handler}: {outcome}");
                    assert_eq!(
                        terminal.pending_order_action,
                        Some(PendingOrderAction::ChaseBuy)
                    );
                    assert_eq!(
                        terminal.order_status,
                        Some(("existing status".to_string(), false))
                    );
                    if has_chase {
                        let chase = chase_from_terminal(&terminal, 1);
                        assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
                        assert_eq!(chase.current_cloid.as_deref(), Some(TEST_CLOID));
                        assert_eq!(chase.current_oid, None);
                        assert!(chase.known_oids.is_empty());
                        assert_eq!(chase.filled_size, 0.0);
                        assert_eq!(chase.remaining_size, 1.0);
                        assert_eq!(chase.desired_price, Some(101.0));
                        assert_eq!(chase.cancel_retries, 2);
                        assert_eq!(chase.last_reprice_at, None);
                        assert_eq!(chase.stop_reason, None);
                    } else {
                        assert!(terminal.chase_orders.is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn chase_placement_check_preserves_fallback_identity_and_stopping_state() {
    for state in [
        "absent",
        "no identifiers",
        "oid only",
        "cloid",
        "stopping cloid",
    ] {
        for connected_to_owner in [false, true] {
            let mut chase = chase();
            chase.lifecycle = ChaseLifecycle::Placing;
            if state == "oid only" {
                chase.current_oid = Some(9001);
            }
            let has_cloid = matches!(state, "cloid" | "stopping cloid");
            if has_cloid {
                chase.current_cloid = Some(TEST_CLOID.to_string());
            }
            if state == "stopping cloid" {
                chase.lifecycle = ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::AwaitingPlace,
                };
                chase.stop_reason = Some(("user stopped".to_string(), false));
            }
            let mut terminal = terminal_with_chase(chase);
            if state == "absent" {
                terminal.chase_orders.clear();
            }
            terminal.connected_address = Some(
                if connected_to_owner {
                    TEST_ACCOUNT
                } else {
                    OTHER_ACCOUNT
                }
                .to_string(),
            );
            terminal.account_loading = false;

            let task = terminal.check_chase_place_status_by_cloid(
                1,
                "unconfirmed: api_key=fixture-secret".to_string(),
            );

            let refresh = state != "absent" && connected_to_owner;
            assert_eq!(terminal.account_loading, refresh, "{state}");
            assert_eq!(task.units() > 0, has_cloid || refresh, "{state}");
            let (message, is_error) = terminal.order_status.as_ref().expect("failure status");
            assert!(*is_error);
            assert!(message.contains("api_key=<redacted>"));
            assert!(!message.contains("fixture-secret"));
            assert_eq!(message.contains("no cloid available"), !has_cloid);
            if matches!(state, "absent" | "no identifiers") {
                assert!(terminal.chase_orders.is_empty());
            } else {
                let chase = chase_from_terminal(&terminal, 1);
                let expected = match state {
                    "oid only" => ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    },
                    "cloid" => ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::Placement,
                    },
                    _ => ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::AwaitingPlace,
                    },
                };
                assert_eq!(chase.lifecycle, expected);
                assert_eq!(chase.last_reprice_at.is_some(), has_cloid);
                assert_eq!(chase.filled_size, 0.0);
                assert_eq!(chase.remaining_size, 1.0);
                if state == "stopping cloid" {
                    assert_eq!(chase.stop_reason, Some(("user stopped".to_string(), false)));
                }
            }
        }
    }
}
