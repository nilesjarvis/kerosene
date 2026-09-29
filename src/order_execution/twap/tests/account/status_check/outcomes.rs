use super::*;
use crate::api::OrderStatusResult;
use crate::twap_state::{TwapEventKind, TwapOrder, TwapPendingSlice};

#[test]
fn twap_status_updates_all_matching_children_and_preserves_fill_details() {
    for (status, exhausted, expected_child_status) in [
        ("unknownOid", false, TwapChildStatus::StatusUnknown),
        ("unknownOid", true, TwapChildStatus::StatusUnknown),
        ("rejected", false, TwapChildStatus::Rejected),
        (
            "canceled",
            false,
            TwapChildStatus::AwaitingNoFillConfirmation,
        ),
        ("open", false, TwapChildStatus::UnexpectedResting),
        ("filled", false, TwapChildStatus::AwaitingReconciliation),
    ] {
        for stop_requested in [false, true] {
            for oid in [None, Some(777)] {
                let now = Instant::now();
                let mut terminal = origin_account_terminal();
                let mut twap = test_twap(1, CLOID, now);
                twap.stop_requested = stop_requested;
                twap.stop_reason = stop_requested.then(|| ("user stopped".to_string(), false));
                twap.status_check_retries = if exhausted {
                    TWAP_MAX_RETRY_ATTEMPTS - 1
                } else {
                    0
                };
                twap.filled_size = 0.25;
                twap.remaining_size = 0.75;
                let template = twap.child_orders[0].clone();
                twap.child_orders.clear();
                for (index, cloid) in [
                    Some(CLOID.to_string()),
                    Some(CLOID.to_string()),
                    Some("unrelated".to_string()),
                    Some(CLOID.to_ascii_uppercase()),
                    None,
                ]
                .into_iter()
                .enumerate()
                {
                    let mut child = template.clone();
                    child.index = index as u32 + 1;
                    child.oid = Some(10 + index as u64);
                    child.cloid = cloid;
                    child.filled_size = 0.1;
                    child.avg_price = Some(100.0);
                    child.fee = 0.01;
                    twap.child_orders.push(child);
                }
                terminal.twap_orders.insert(1, twap);
                let summary = format!("{status} fixture");

                let _task = terminal.handle_twap_order_status_result(
                    1,
                    CLOID.to_string(),
                    Ok(OrderStatusResult {
                        status: status.to_string(),
                        oid,
                        cloid: Some(CLOID.to_string()),
                        raw_summary: summary.clone(),
                    }),
                );

                let twap = twap_by_id(&terminal, 1);
                assert_eq!(twap.filled_size, 0.25);
                assert_eq!(twap.remaining_size, 0.75);
                for (index, child) in twap.child_orders.iter().enumerate() {
                    assert_eq!(child.filled_size, 0.1);
                    assert_eq!(child.avg_price, Some(100.0));
                    assert_eq!(child.fee, 0.01);
                    if index < 2 {
                        let expected = if status == "unknownOid" && exhausted && stop_requested {
                            TwapChildStatus::NoFill
                        } else {
                            expected_child_status
                        };
                        assert_eq!(
                            child.status, expected,
                            "{status}, stopping: {stop_requested}"
                        );
                        assert_eq!(child.oid, oid.or(Some(10 + index as u64)));
                        assert_eq!(child.cloid.as_deref(), Some(CLOID));
                        assert_eq!(child.exchange_summary, summary);
                    } else {
                        assert_eq!(child.status, TwapChildStatus::StatusUnknown);
                        assert_eq!(child.oid, Some(10 + index as u64));
                        assert_eq!(child.exchange_summary, "status unknown");
                    }
                }
            }
        }
    }
}

#[test]
fn twap_unrecognized_status_and_transport_failure_keep_distinct_retry_feedback() {
    for transport in [false, true] {
        for attempts in [0, TWAP_MAX_RETRY_ATTEMPTS - 1, u32::MAX] {
            for stop_requested in [false, true] {
                let now = Instant::now();
                let mut terminal = switched_account_terminal();
                terminal.account_loading = false;
                terminal.order_status = Some(("existing status".to_string(), false));
                let mut twap = test_twap(1, CLOID, now);
                twap.status_check_retries = attempts;
                twap.stop_requested = stop_requested;
                twap.paused_until = Some(now);
                let retry_slice = TwapPendingSlice {
                    index: 1,
                    planned_size: 0.5,
                    limit_price: 100.0,
                    cloid: "retry-cloid".to_string(),
                    retry_count: 2,
                };
                twap.retry_slice = Some(retry_slice.clone());
                terminal.twap_orders.insert(1, twap);
                let result = if transport {
                    Err("network unavailable".to_string())
                } else {
                    Ok(OrderStatusResult {
                        status: "mystery".to_string(),
                        oid: Some(777),
                        cloid: Some(CLOID.to_string()),
                        raw_summary: "mystery fixture".to_string(),
                    })
                };

                let task = terminal.handle_twap_order_status_result(1, CLOID.to_string(), result);

                let twap = twap_by_id(&terminal, 1);
                let attempt = attempts.saturating_add(1);
                let exhausted = attempt >= TWAP_MAX_RETRY_ATTEMPTS;
                assert_eq!(twap.status_check_retries, attempt);
                assert_eq!(twap.retry_slice, Some(retry_slice));
                assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
                assert_eq!(twap.child_orders[0].oid, None);
                assert_eq!(twap.child_orders[0].exchange_summary, "status unknown");
                assert!(!terminal.account_loading);
                assert_eq!(
                    terminal.order_status,
                    Some(("existing status".to_string(), false))
                );
                assert_eq!(task.units(), if exhausted { 0 } else { 1 });
                let event = twap.events.last().expect("retry feedback");
                assert!(event.is_error);
                if exhausted {
                    assert_eq!(twap.status, TwapStatus::Error);
                    assert_eq!(twap.status_check_cloid, None);
                    assert_eq!(twap.paused_until, Some(now));
                    assert_eq!(event.kind, TwapEventKind::Error);
                    let expected = if transport {
                        format!(
                            "Could not check slice status after {TWAP_MAX_RETRY_ATTEMPTS} attempts: network unavailable"
                        )
                    } else {
                        format!("Could not reconcile slice {CLOID} after status 'mystery'")
                    };
                    assert_eq!(event.message, expected);
                    assert_eq!(terminal.advanced_order_history.len(), 1);
                } else {
                    assert_eq!(
                        twap.status,
                        if stop_requested {
                            TwapStatus::Stopping
                        } else {
                            TwapStatus::Paused
                        }
                    );
                    assert_eq!(twap.status_check_cloid.as_deref(), Some(CLOID));
                    assert_eq!(
                        twap.pause_reason,
                        Some(if transport {
                            TwapPauseReason::NetworkError
                        } else {
                            TwapPauseReason::StatusUnknown
                        })
                    );
                    let delay = TwapOrder::retry_delay(attempt);
                    assert!(
                        twap.paused_until
                            .is_some_and(|at| at >= now + delay && at <= Instant::now() + delay)
                    );
                    let seconds = delay.as_secs();
                    let expected = if transport {
                        format!(
                            "Slice status check failed; retry {attempt}/{TWAP_MAX_RETRY_ATTEMPTS} in about {seconds}s: network unavailable"
                        )
                    } else {
                        format!(
                            "Slice status still unclear (mystery); retry {attempt}/{TWAP_MAX_RETRY_ATTEMPTS} in about {seconds}s"
                        )
                    };
                    assert_eq!(event.message, expected);
                    assert!(terminal.advanced_order_history.is_empty());
                }
            }
        }
    }
}

#[test]
fn twap_status_results_ignore_absent_stale_and_terminal_orders() {
    for state in ["absent", "no check", "different check", "terminal"] {
        let mut terminal = origin_account_terminal();
        terminal.account_loading = false;
        terminal.order_status = Some(("existing status".to_string(), false));
        let mut twap = test_twap(1, CLOID, Instant::now());
        twap.status_check_cloid = match state {
            "no check" => None,
            "different check" => Some("other-cloid".to_string()),
            _ => Some(CLOID.to_string()),
        };
        if state == "terminal" {
            twap.status = TwapStatus::Error;
        }
        let expected_check = twap.status_check_cloid.clone();
        let expected_status = twap.status;
        if state != "absent" {
            terminal.twap_orders.insert(1, twap);
        }

        let task = terminal.handle_twap_order_status_result(
            1,
            CLOID.to_string(),
            Ok(filled_status(CLOID, CHILD_OID)),
        );

        assert_eq!(task.units(), 0);
        assert!(!terminal.account_loading);
        assert!(terminal.advanced_order_history.is_empty());
        assert_eq!(
            terminal.order_status,
            Some(("existing status".to_string(), false))
        );
        if state != "absent" {
            let twap = twap_by_id(&terminal, 1);
            assert_eq!(twap.status_check_cloid, expected_check);
            assert_eq!(twap.status, expected_status);
            assert_eq!(twap.child_orders[0].status, TwapChildStatus::StatusUnknown);
            assert_eq!(twap.reconciliation_deadline, None);
        }
    }
}
