use super::fixtures::{test_twap, twap_by_id};
use crate::api::{BookLevel, OrderBook};
use crate::app_state::TradingTerminal;
use crate::twap_state::{
    TWAP_BOOK_STALE_AFTER, TwapBookSnapshot, TwapChildStatus, TwapEventKind, TwapOrder,
    TwapPauseReason, TwapPendingOp, TwapPendingSlice, TwapStatus, twap_child_cloid,
};

use std::time::{Duration, Instant};

const INITIAL_SEED: u64 = 123_456_789;
const NEXT_SEED: u64 = 132_100_702_508_589_007;
const RETRY_CLOID: &str = "0x1234567890abcdef1234567890abcdef";

fn ready_terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    terminal.order_status = None;
    terminal
}

fn ready_twap(now: Instant, retry: bool) -> TwapOrder {
    let mut twap = test_twap(1, RETRY_CLOID, now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    twap.random_seed = INITIAL_SEED;
    twap.events.clear();
    twap.latest_book = Some(TwapBookSnapshot {
        book: OrderBook {
            bids: vec![
                BookLevel { px: 99.0, sz: 0.2 },
                BookLevel { px: 98.0, sz: 1.0 },
            ],
            asks: vec![
                BookLevel { px: 101.0, sz: 0.2 },
                BookLevel { px: 102.0, sz: 1.0 },
            ],
        },
        updated_at: now,
    });
    if retry {
        twap.slices_attempted = 1;
        twap.slices_sent = 1;
        twap.status = TwapStatus::Paused;
        twap.pause_reason = Some(TwapPauseReason::NetworkError);
        twap.paused_until = Some(now);
        twap.retry_slice = Some(TwapPendingSlice {
            index: 1,
            planned_size: 0.437,
            limit_price: 100.0,
            cloid: RETRY_CLOID.to_string(),
            retry_count: 2,
        });
        twap.child_orders[0].status = TwapChildStatus::NoFill;
        twap.child_orders[0].oid = Some(77);
        twap.child_orders[0].filled_size = 0.125;
        twap.child_orders[0].avg_price = Some(100.0);
        twap.child_orders[0].fee = 0.01;
    } else {
        twap.child_orders.clear();
    }
    twap
}

#[test]
fn twap_slice_dispatch_preserves_sizing_retry_identity_and_cached_book() {
    for retry in [false, true] {
        for randomize in [false, true] {
            for is_buy in [false, true] {
                let now = Instant::now();
                let mut terminal = ready_terminal();
                let mut twap = ready_twap(now, retry);
                twap.is_buy = is_buy;
                twap.randomize = randomize;
                let expected_cloid = if retry {
                    RETRY_CLOID.to_string()
                } else {
                    twap_child_cloid(&twap.account_address, twap.id, twap.started_at_ms, 1)
                };
                terminal.twap_orders.insert(1, twap);

                // Inspect state and task count without polling the signing/network future.
                let task = terminal.execute_due_twap_slice(1, now);

                assert_eq!(task.units(), 1);
                let twap = twap_by_id(&terminal, 1);
                let expected_size = if retry {
                    0.437
                } else if randomize {
                    0.401
                } else {
                    0.5
                };
                let expected_price = if is_buy { 102.0 } else { 98.0 };
                assert_eq!(
                    twap.pending_op,
                    Some(TwapPendingOp::Place(TwapPendingSlice {
                        index: 1,
                        planned_size: expected_size,
                        limit_price: expected_price,
                        cloid: expected_cloid.clone(),
                        retry_count: if retry { 2 } else { 0 },
                    }))
                );
                assert!(twap.retry_slice.is_none());
                assert_eq!(
                    twap.random_seed,
                    if randomize && !retry {
                        NEXT_SEED
                    } else {
                        INITIAL_SEED
                    }
                );
                assert_eq!((twap.slices_attempted, twap.slices_sent), (1, 1));
                assert_eq!(twap.status, TwapStatus::Running);
                assert_eq!(twap.pause_reason, None);
                assert_eq!(twap.paused_until, None);
                assert_eq!(twap.remaining_size, 1.0);
                assert_eq!(twap.filled_size, 0.0);
                assert_eq!(twap.child_orders.len(), 1);
                let child = &twap.child_orders[0];
                assert_eq!(child.cloid.as_deref(), Some(expected_cloid.as_str()));
                assert_eq!(child.status, TwapChildStatus::Pending);
                assert_eq!(child.limit_price, expected_price);
                assert_eq!(child.retry_count, if retry { 2 } else { 0 });
                assert_eq!(
                    child.exchange_summary,
                    if retry { "Retry 2" } else { "Placing" }
                );
                assert_eq!(child.planned_size, if retry { 0.5 } else { expected_size });
                assert_eq!(child.oid, retry.then_some(77));
                assert_eq!(child.filled_size, if retry { 0.125 } else { 0.0 });
                assert_eq!(child.avg_price, retry.then_some(100.0));
                assert_eq!(child.fee, if retry { 0.01 } else { 0.0 });
                let snapshot = twap
                    .latest_book
                    .as_ref()
                    .expect("cached book remains available");
                assert_eq!(snapshot.updated_at, now);
                for (levels, expected) in [
                    (&snapshot.book.bids, [(99.0, 0.2), (98.0, 1.0)]),
                    (&snapshot.book.asks, [(101.0, 0.2), (102.0, 1.0)]),
                ] {
                    assert_eq!(levels.len(), expected.len());
                    for (level, (price, size)) in levels.iter().zip(expected) {
                        assert_eq!((level.px, level.sz), (price, size));
                    }
                }
                assert_eq!(twap.events.len(), 1);
                assert_eq!(
                    twap.events[0].kind,
                    if retry {
                        TwapEventKind::Retrying
                    } else {
                        TwapEventKind::Placed
                    }
                );
                assert!(!twap.events[0].is_error);
                assert_eq!(terminal.last_advanced_exchange_request_at, Some(now));
                assert!(terminal.order_status.is_none());
            }
        }
    }
}

#[test]
fn twap_slice_gates_retain_retry_plan_and_defer_random_sizing() {
    for gate in [
        "missing book",
        "stale book",
        "loading",
        "reconciling",
        "throttled",
        "missing key",
    ] {
        for retry in [false, true] {
            let now = Instant::now();
            let mut terminal = ready_terminal();
            let mut twap = ready_twap(now, retry);
            twap.randomize = true;
            match gate {
                "missing book" => twap.latest_book = None,
                "stale book" => {
                    twap.latest_book.as_mut().expect("fixture book").updated_at =
                        now - TWAP_BOOK_STALE_AFTER - Duration::from_nanos(1);
                    // Staleness feedback must precede the loading gate.
                    terminal.account_loading = true;
                }
                "loading" => terminal.account_loading = true,
                "reconciling" => terminal.account_reconciliation_required = true,
                "throttled" => terminal.last_advanced_exchange_request_at = Some(now),
                "missing key" => twap.agent_key.clear(),
                _ => unreachable!("known gate fixture"),
            }
            let retry_plan = twap.retry_slice.clone();
            let last_dispatch = terminal.last_advanced_exchange_request_at;
            terminal.twap_orders.insert(1, twap);

            let task = terminal.execute_due_twap_slice(1, now);

            assert_eq!(task.units(), 0, "{gate}, retry: {retry}");
            let twap = twap_by_id(&terminal, 1);
            assert_eq!(twap.retry_slice, retry_plan, "{gate}");
            assert!(twap.pending_op.is_none());
            assert_eq!(
                (twap.slices_attempted, twap.slices_sent),
                (u32::from(retry), u32::from(retry))
            );
            assert_eq!(
                twap.random_seed,
                if gate == "missing key" && !retry {
                    NEXT_SEED
                } else {
                    INITIAL_SEED
                },
                "{gate}"
            );
            assert_eq!(terminal.last_advanced_exchange_request_at, last_dispatch);
            let expected_status = match gate {
                "missing key" => TwapStatus::Stopped,
                "stale book" => TwapStatus::Paused,
                "missing book" if !retry => TwapStatus::WaitingForMarket,
                _ if retry => TwapStatus::Paused,
                _ => TwapStatus::Running,
            };
            assert_eq!(twap.status, expected_status, "{gate}");
            let expected_feedback = match gate {
                "stale book" => Some("TWAP paused: market data is stale"),
                "missing key" => Some("TWAP stopped: original agent key is unavailable"),
                _ => None,
            };
            assert_eq!(
                terminal.order_status,
                expected_feedback.map(|message| (message.to_string(), true))
            );
            assert_eq!(twap.events.len(), usize::from(expected_feedback.is_some()));
            if retry {
                assert_eq!(twap.child_orders[0].status, TwapChildStatus::NoFill);
                assert_eq!(twap.child_orders[0].limit_price, 100.0);
                assert_eq!(twap.child_orders[0].retry_count, 0);
            }
        }
    }
}

#[test]
fn twap_slice_skips_account_for_new_attempts_and_existing_retries() {
    for minimum_notional in [false, true] {
        for retry in [false, true] {
            let now = Instant::now();
            let mut terminal = ready_terminal();
            let mut twap = ready_twap(now, retry);
            if minimum_notional {
                twap.remaining_size = 0.1;
                if let Some(slice) = &mut twap.retry_slice {
                    slice.planned_size = 0.05;
                }
            } else {
                twap.max_price = 100.0;
            }
            terminal.twap_orders.insert(1, twap);

            let task = terminal.execute_due_twap_slice(1, now);

            assert_eq!(task.units(), 0);
            let twap = twap_by_id(&terminal, 1);
            assert!(twap.retry_slice.is_none());
            assert!(twap.pending_op.is_none());
            assert_eq!(twap.slices_attempted, 1);
            assert_eq!(twap.slices_sent, u32::from(retry));
            assert_eq!(twap.status, TwapStatus::WaitingForMarket);
            assert_eq!(twap.pause_reason, None);
            assert_eq!(twap.paused_until, None);
            assert_eq!(twap.next_slice_due, now + Duration::from_secs(300));
            assert_eq!(twap.random_seed, INITIAL_SEED);
            let message = if minimum_notional {
                "TWAP slice skipped: child notional $5.05 is below Hyperliquid's $10 minimum"
            } else if retry {
                "TWAP slice skipped: book cannot fill 0.437 inside 90.00-100.00"
            } else {
                "TWAP slice skipped: book cannot fill 0.5 inside 90.00-100.00"
            };
            assert_eq!(
                terminal.order_status,
                Some((message.to_string(), minimum_notional))
            );
            assert_eq!(twap.events.len(), 1);
            assert_eq!(
                twap.events[0].kind,
                if minimum_notional {
                    TwapEventKind::SkippedMinimum
                } else {
                    TwapEventKind::SkippedRange
                }
            );
            assert_eq!(twap.events[0].message, message);
            assert_eq!(twap.events[0].is_error, minimum_notional);
            assert_eq!(twap.child_orders.len(), usize::from(retry));
            if retry {
                assert_eq!(twap.child_orders[0].status, TwapChildStatus::NoFill);
                assert_eq!(twap.child_orders[0].exchange_summary, message);
                assert_eq!(twap.child_orders[0].cloid.as_deref(), Some(RETRY_CLOID));
                assert_eq!(twap.child_orders[0].filled_size, 0.125);
            }
            assert!(terminal.last_advanced_exchange_request_at.is_none());
        }
    }
}
