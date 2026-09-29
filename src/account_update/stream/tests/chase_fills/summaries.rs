use super::{
    ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason, chase_order, chase_order_by_id,
    connected_terminal_with_chase_account, fill_with_oid, open_order,
};
use crate::account_update::stream::fills::chase_fill_totals_for_chase;
use crate::advanced_order_history::AdvancedOrderHistoryEntry;

#[test]
fn chase_fill_membership_preserves_distinct_live_and_history_filters() {
    let mut fills = vec![
        fill_with_oid(100, 42, "10", "2"),
        fill_with_oid(101, 42, "20", "4"),
        fill_with_oid(102, 42, "30", "1"),
        fill_with_oid(103, 42, "40", "3"),
        fill_with_oid(104, 42, "50", "4"),
        fill_with_oid(105, 42, "60", "NaN"),
        fill_with_oid(106, 42, "50", "999"),
        fill_with_oid(107, 43, "70", "5"),
        fill_with_oid(108, 43, "80", "6"),
        fill_with_oid(109, 43, "NaN", "1"),
        fill_with_oid(110, 99, "999", "999"),
    ];
    for (index, fill) in fills.iter_mut().enumerate() {
        fill.tid = Some(index as u64 + 1);
        fill.fee = "1".into();
        fill.fee_token = Some("USDC".into());
        fill.closed_pnl = "2".into();
    }
    fills[1].coin = "OTHER".into();
    fills[2].tid = fills[1].tid; // Live filtering precedes deduplication.
    fills[3].side = "A".into();
    fills[4].side = "unknown".into();
    fills[5].fee = "2".into();
    fills[5].fee_token = Some("BTC".into());
    fills[6].tid = fills[5].tid; // Invalid size still consumes the duplicate ID.
    fills[8].side = "A".into();
    fills[8].fee = "-1".into();
    fills[8].closed_pnl = "-2".into();
    fills[9].fee_token = Some("BTC".into());
    fills[9].closed_pnl = "1".into();
    let mut missing_oid = fills[10].clone();
    missing_oid.oid = None;
    fills.push(missing_oid);

    let cases = [
        (vec![], None, false, false),
        (vec![999], Some(998), false, false),
        (vec![], Some(42), true, false),
        (vec![42], None, true, false),
        (vec![42], Some(42), true, false),
        (vec![42, 42], Some(43), true, true),
        (vec![43], Some(42), true, true),
        (vec![43], None, false, true),
        (vec![42], Some(999), true, false),
    ];
    for is_buy in [false, true] {
        for (known_oids, current_oid, includes_42, includes_43) in &cases {
            let mut chase = chase_order();
            chase.is_buy = is_buy;
            chase.known_oids = known_oids.clone();
            chase.current_oid = *current_oid;
            chase.fill_cutoff_ms_by_oid = vec![(42, 101)];
            let live = chase_fill_totals_for_chase(&fills, &chase);
            let history = AdvancedOrderHistoryEntry::chase_fill_metrics(&fills, &chase);
            if !includes_42 && !includes_43 {
                assert!(live.is_none());
                assert!(history.is_none());
                continue;
            }
            let live = live.expect("matching live fills");
            let history = history.expect("matching historical fills");
            let (size_42, notional_42, size_43, notional_43) = if is_buy {
                (1.0, 30.0, 5.0, 350.0)
            } else {
                (3.0, 120.0, 6.0, 480.0)
            };
            let count_42 = u8::from(*includes_42) as f64;
            let count_43 = u8::from(*includes_43) as f64;
            assert_eq!(live.side, if is_buy { "BUY" } else { "SELL" });
            assert_eq!(live.filled_size, count_42 * size_42 + count_43 * size_43);
            assert_eq!(
                live.total_notional,
                count_42 * notional_42 + count_43 * notional_43
            );
            assert_eq!(history.filled_size, count_42 * 13.0 + count_43 * 11.0);
            assert_eq!(history.gross_notional, count_42 * 420.0 + count_43 * 830.0);
            assert_eq!(history.total_fee, count_42 * 124.0);
            assert_eq!(history.closed_pnl, count_42 * 10.0 + count_43);
        }
    }

    let invalid = fill_with_oid(100, 42, "100", "NaN");
    let chase = chase_order();
    let live = chase_fill_totals_for_chase(std::slice::from_ref(&invalid), &chase)
        .expect("matched invalid numbers still produce totals");
    assert_eq!((live.filled_size, live.total_notional), (0.0, 0.0));
    let history = AdvancedOrderHistoryEntry::chase_fill_metrics(&[invalid], &chase)
        .expect("matched invalid numbers still produce metrics");
    assert_eq!((history.filled_size, history.gross_notional), (0.0, 0.0));
    assert_eq!(history.total_fee, 0.01);
}

#[test]
fn chase_completion_summary_uses_matched_totals_when_recorded_fills_differ() {
    let cases = [
        (1.0, 0.0, "1", Some("1"), false),
        (1.0, 0.0, "1.25", Some("1.25"), true),
        (1.0, 2.0, "0.5", Some("0.5"), false),
        (1.0, 1.0, "NaN", None, false),
        (0.0, 0.0, "NaN", None, false),
        (f64::NAN, 0.0, "NaN", None, false),
    ];
    for is_buy in [false, true] {
        for open_orders_state in 0..3 {
            for (target_size, recorded_filled, size, expected_size, is_error) in cases {
                let mut chase = chase_order();
                chase.is_buy = is_buy;
                chase.target_size = target_size;
                chase.filled_size = recorded_filled;
                let mut fill = fill_with_oid(1_001, 42, "100", size);
                fill.side = if is_buy { "B" } else { "A" }.into();
                let mut resting = open_order(42, Some(false));
                resting.side = fill.side.clone();
                let mut terminal = connected_terminal_with_chase_account(
                    chase,
                    vec![fill.clone(), fill],
                    if open_orders_state == 2 {
                        vec![resting]
                    } else {
                        vec![]
                    },
                );
                terminal.account_loading = false;
                terminal.account_reconciliation_required = false;
                terminal
                    .account_data
                    .as_mut()
                    .expect("account")
                    .completeness
                    .open_orders_complete = open_orders_state != 0;

                let task = terminal.reconcile_chase_fills_from_account();

                let order = chase_order_by_id(&terminal, 1);
                let mut expected = if let Some(size) = expected_size {
                    format!(
                        "Chase filled: {} {size} BTC @ $100",
                        if is_buy { "BUY" } else { "SELL" }
                    )
                } else {
                    "Chase filled".into()
                };
                if is_error {
                    expected.push_str("; over target by 0.25");
                }
                assert_eq!(order.stop_reason, Some((expected.clone(), is_error)));
                assert!(terminal.advanced_order_history.is_empty());
                if open_orders_state == 2 {
                    assert_eq!(task.units(), 1);
                    assert_eq!(
                        order.lifecycle,
                        ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::Canceling { oid: 42 }
                        }
                    );
                    assert_eq!(
                        terminal.order_status,
                        Some((format!("{expected}: cancelling order 42"), is_error))
                    );
                } else {
                    assert_eq!(
                        order.lifecycle,
                        ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder
                        }
                    );
                    assert!(terminal.account_loading);
                    assert!(terminal.account_reconciliation_required);
                }
            }
        }
    }
}
