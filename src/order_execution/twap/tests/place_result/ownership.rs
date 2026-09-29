use super::*;
use crate::twap_state::{TwapPendingOp, TwapPendingSlice};

#[test]
fn late_twap_slice_results_preserve_other_pending_operations() {
    let pending_ops = [
        None,
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: None,
            cloid: None,
        }),
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(77),
            cloid: None,
        }),
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: None,
            cloid: Some("original-cloid".to_string()),
        }),
        Some(TwapPendingOp::CancelUnexpectedResting {
            oid: Some(77),
            cloid: Some("original-cloid".to_string()),
        }),
    ];
    for pending in pending_ops {
        for terminal_status in [false, true] {
            for connected_to_owner in [false, true] {
                for outcome in ["filled", "rejected", "transport"] {
                    let now = Instant::now();
                    let mut twap = pending_twap(1, "original-cloid", now);
                    twap.pending_op = pending.clone();
                    twap.status = if terminal_status {
                        TwapStatus::Error
                    } else {
                        TwapStatus::Running
                    };
                    twap.retry_slice = Some(TwapPendingSlice {
                        index: 1,
                        planned_size: 0.4,
                        limit_price: 99.0,
                        cloid: "retry-cloid".to_string(),
                        retry_count: 2,
                    });
                    let retry = twap.retry_slice.clone();
                    let events = twap.events.len();
                    let mut terminal = TradingTerminal::boot().0;
                    terminal.connected_address =
                        Some(if connected_to_owner { "0xabc" } else { "0xdef" }.to_string());
                    terminal.account_loading = false;
                    terminal.order_status = Some(("existing status".to_string(), false));
                    terminal.twap_orders.insert(1, twap);
                    let result = match outcome {
                        "filled" => Ok(exchange_response(
                            serde_json::json!({"filled": {"oid": 77, "totalSz": "0.5", "avgPx": "100"}}),
                        )),
                        "rejected" => Ok(exchange_response(
                            serde_json::json!({"error": "tick rejected"}),
                        )),
                        _ => Err("connection closed".to_string()),
                    };

                    let task = terminal.handle_twap_slice_result(1, result);

                    let twap = twap_by_id(&terminal, 1);
                    assert_eq!(twap.pending_op, pending);
                    assert_eq!(twap.retry_slice, retry);
                    assert_eq!(
                        twap.status,
                        if terminal_status {
                            TwapStatus::Error
                        } else {
                            TwapStatus::Running
                        }
                    );
                    assert_eq!(twap.slices_attempted, 0);
                    assert_eq!(twap.filled_size, 0.0);
                    assert_eq!(twap.remaining_size, 1.0);
                    assert_eq!(twap.status_check_cloid, None);
                    assert_eq!(twap.child_orders[0].status, TwapChildStatus::Pending);
                    assert_eq!(twap.child_orders[0].oid, None);
                    assert_eq!(twap.events.len(), events);
                    assert!(terminal.advanced_order_history.is_empty());
                    assert_eq!(
                        terminal.order_status,
                        Some(("existing status".to_string(), false))
                    );
                    let refresh =
                        outcome == "transport" || (outcome == "filled" && terminal_status);
                    assert_eq!(task.units() > 0, refresh);
                    assert_eq!(terminal.account_loading, refresh && connected_to_owner);
                }
            }
        }
    }
}

#[test]
fn twap_slice_results_retain_child_and_retry_identity() {
    for outcome in [
        "retry",
        "filled",
        "filled without size",
        "resting",
        "ambiguous",
        "transport",
    ] {
        let now = Instant::now();
        let mut twap = pending_twap(1, "original-cloid", now);
        let pending = TwapPendingSlice {
            index: 7,
            planned_size: 0.4,
            limit_price: 104.0,
            cloid: "original-cloid".to_string(),
            retry_count: 2,
        };
        twap.pending_op = Some(TwapPendingOp::Place(pending.clone()));
        twap.child_orders[0].index = pending.index;
        twap.child_orders[0].planned_size = pending.planned_size;
        twap.child_orders[0].limit_price = pending.limit_price;
        twap.child_orders[0].retry_count = pending.retry_count;
        let mut terminal = TradingTerminal::boot().0;
        terminal.twap_orders.insert(1, twap);
        let result = match outcome {
            "retry" => Ok(exchange_response(
                serde_json::json!({"error": "429 Too Many Requests"}),
            )),
            "filled" => Ok(exchange_response(
                serde_json::json!({"filled": {"oid": 77, "totalSz": "0.2", "avgPx": "103"}}),
            )),
            "filled without size" => Ok(exchange_response(
                serde_json::json!({"filled": {"oid": 77}}),
            )),
            "resting" => Ok(exchange_response(
                serde_json::json!({"resting": {"oid": 77}}),
            )),
            "ambiguous" => Ok(exchange_response(serde_json::Value::Null)),
            _ => Err("connection closed".to_string()),
        };

        let _task = terminal.handle_twap_slice_result(1, result);

        let twap = twap_by_id(&terminal, 1);
        let child = &twap.child_orders[0];
        assert_eq!(child.index, 7);
        assert_eq!(child.cloid.as_deref(), Some("original-cloid"));
        assert_eq!(child.planned_size, 0.4);
        assert_eq!(child.limit_price, 104.0);
        if outcome == "retry" {
            assert_eq!(
                twap.retry_slice,
                Some(TwapPendingSlice {
                    retry_count: 3,
                    ..pending
                })
            );
            assert_eq!(child.status, TwapChildStatus::Retrying);
            assert_eq!(child.retry_count, 3);
        } else {
            assert_eq!(twap.retry_slice, None);
            let expected = match outcome {
                "filled" => TwapChildStatus::Filled,
                "filled without size" => TwapChildStatus::AwaitingReconciliation,
                "resting" => TwapChildStatus::UnexpectedResting,
                _ => TwapChildStatus::StatusUnknown,
            };
            assert_eq!(child.status, expected, "{outcome}");
        }
        assert_eq!(
            twap.filled_size,
            if outcome == "filled" { 0.2 } else { 0.0 }
        );
        assert_eq!(
            twap.remaining_size,
            if outcome == "filled" { 0.8 } else { 1.0 }
        );
        if outcome == "resting" {
            assert_eq!(
                twap.pending_op,
                Some(TwapPendingOp::CancelUnexpectedResting {
                    oid: Some(77),
                    cloid: Some("original-cloid".to_string())
                })
            );
        } else {
            assert_eq!(twap.pending_op, None);
        }
        let needs_status = matches!(outcome, "filled without size" | "ambiguous" | "transport");
        assert_eq!(
            twap.status_check_cloid.as_deref(),
            needs_status.then_some("original-cloid")
        );
    }
}
