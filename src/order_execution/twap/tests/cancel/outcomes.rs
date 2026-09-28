use super::*;
use crate::helpers::redact_sensitive_response_text;
use crate::twap_state::{TwapEventKind, TwapOrder};

#[test]
fn twap_cancel_outcomes_preserve_resolution_retry_and_stop_rules() {
    for outcome in [
        "confirmed",
        "already closed",
        "ambiguous",
        "rejected",
        "transport",
        "closed transport",
    ] {
        for attempts in [0, TWAP_MAX_UNEXPECTED_CANCEL_RETRIES - 1, u32::MAX] {
            for stop_requested in [false, true] {
                for connected_to_owner in [false, true] {
                    let mut terminal = terminal_with_unexpected_cancel();
                    terminal.connected_address =
                        Some(if connected_to_owner { "0xabc" } else { "0xdef" }.to_string());
                    terminal.account_loading = false;
                    {
                        let twap = terminal.twap_orders.get_mut(&1).expect("TWAP fixture");
                        twap.cancel_retries = attempts;
                        twap.stop_requested = stop_requested;
                        twap.stop_reason =
                            stop_requested.then(|| ("user stopped".to_string(), false));
                        twap.filled_size = 0.25;
                        twap.remaining_size = 0.75;
                        let mut other_child = twap.child_orders[0].clone();
                        other_child.index = 2;
                        other_child.oid = Some(OID + 1);
                        other_child.cloid = Some("unrelated-cloid".to_string());
                        other_child.exchange_summary = "keep summary".to_string();
                        twap.child_orders.push(other_child);
                    }
                    let result = match outcome {
                        "confirmed" => Ok(cancel_success_response()),
                        "already closed" | "rejected" => Ok(exchange_response_from_value(
                            serde_json::json!({"status": "ok", "response": {"type": "cancel", "data": {"statuses": [{"error": if outcome == "already closed" { "order not found" } else { "invalid signature" }}]}}}),
                            "cancel error response",
                        )),
                        "ambiguous" => Ok(empty_cancel_response()),
                        "transport" => Err("connection closed: token=fixture-secret".to_string()),
                        _ => Err("order not found: token=fixture-secret".to_string()),
                    };
                    let summary = match &result {
                        Ok(response) => response.summary(),
                        Err(error) => redact_sensitive_response_text(error),
                    };
                    let before = Instant::now();

                    let task = terminal.handle_twap_unexpected_cancel_result(
                        1,
                        Some(OID),
                        Some(CLOID.to_string()),
                        result,
                    );

                    let twap = twap_by_id(&terminal, 1);
                    assert_eq!(twap.filled_size, 0.25);
                    assert_eq!(twap.remaining_size, 0.75);
                    assert_eq!(twap.child_orders[0].exchange_summary, summary);
                    assert_eq!(twap.child_orders[1].exchange_summary, "keep summary");
                    assert_eq!(
                        twap.child_orders[1].status,
                        TwapChildStatus::UnexpectedResting
                    );
                    assert!(task.units() > 0);
                    assert_eq!(terminal.account_loading, connected_to_owner);
                    let resolved = matches!(outcome, "confirmed" | "already closed");
                    let exhausted =
                        attempts.saturating_add(1) >= TWAP_MAX_UNEXPECTED_CANCEL_RETRIES;
                    if resolved {
                        assert_eq!(twap.pending_op, None);
                        assert_eq!(twap.cancel_retries, 0);
                        assert_eq!(twap.pause_reason, None);
                        assert_eq!(
                            twap.child_orders[0].status,
                            TwapChildStatus::UnexpectedRestingCancelled
                        );
                        let event = twap
                            .events
                            .iter()
                            .find(|event| event.kind == TwapEventKind::Reconciled)
                            .expect("reconciled event");
                        assert_eq!(event.is_error, outcome == "already closed");
                        let expected = if outcome == "confirmed" {
                            format!("Canceled unexpected resting child oid {OID} / {CLOID}")
                        } else {
                            format!(
                                "Unexpected resting child oid {OID} / {CLOID} is no longer open: {summary}"
                            )
                        };
                        assert_eq!(event.message, expected);
                        if stop_requested {
                            assert_eq!(twap.status, TwapStatus::Stopped);
                            assert_eq!(
                                terminal.order_status,
                                Some(("user stopped".to_string(), false))
                            );
                        }
                    } else {
                        assert_eq!(twap.cancel_retries, attempts.saturating_add(1));
                        assert_eq!(
                            twap.child_orders[0].status,
                            TwapChildStatus::UnexpectedResting
                        );
                        let transport = outcome.ends_with("transport");
                        let event = twap.events.last().expect("retry or error event");
                        assert!(event.is_error);
                        if exhausted {
                            assert_eq!(twap.pending_op, None);
                            assert_eq!(twap.status, TwapStatus::Error);
                            assert_eq!(event.kind, TwapEventKind::Error);
                            let prefix = if transport {
                                "Cancel status unknown for unexpected child"
                            } else {
                                "Failed to cancel unexpected resting child"
                            };
                            assert_eq!(
                                event.message,
                                format!(
                                    "{prefix} oid {OID} / {CLOID} after {TWAP_MAX_UNEXPECTED_CANCEL_RETRIES} attempts: {summary}"
                                )
                            );
                        } else {
                            assert_eq!(
                                twap.pending_op,
                                Some(TwapPendingOp::CancelUnexpectedResting {
                                    oid: Some(OID),
                                    cloid: Some(CLOID.to_string())
                                })
                            );
                            assert_eq!(
                                twap.status,
                                if stop_requested {
                                    TwapStatus::Stopping
                                } else {
                                    TwapStatus::Paused
                                }
                            );
                            assert_eq!(twap.pause_reason, Some(TwapPauseReason::UnexpectedResting));
                            let delay = TwapOrder::retry_delay(attempts + 1);
                            assert!(twap.paused_until.is_some_and(
                                |at| at >= before + delay && at <= Instant::now() + delay
                            ));
                            let attempt = attempts + 1;
                            let seconds = delay.as_secs();
                            let expected = if transport {
                                format!(
                                    "Cancel status unknown for unexpected child oid {OID} / {CLOID}; retry {attempt}/{TWAP_MAX_UNEXPECTED_CANCEL_RETRIES} in about {seconds}s"
                                )
                            } else {
                                format!(
                                    "Cancel retry {attempt}/{TWAP_MAX_UNEXPECTED_CANCEL_RETRIES} for unexpected resting child oid {OID} / {CLOID} in about {seconds}s"
                                )
                            };
                            assert_eq!(event.message, expected);
                        }
                    }
                    assert_eq!(
                        !terminal.advanced_order_history.is_empty(),
                        resolved && stop_requested || !resolved && exhausted
                    );
                }
            }
        }
    }
}
