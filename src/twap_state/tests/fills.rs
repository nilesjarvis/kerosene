use super::{
    TwapChildOrder, TwapChildStatus, TwapStatus, exchange_response_from_value, test_twap_order,
    twap_response_fill_summary, user_fill,
};

use std::time::Instant;

#[test]
fn twap_fill_summary_does_not_invent_missing_fill_size() {
    let response = exchange_response_from_value(
        serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{
                        "filled": {
                            "avgPx": "100",
                            "oid": 77_u64
                        }
                    }]
                }
            }
        }),
        "test exchange response should deserialize",
    );

    assert!(response.is_fully_filled());
    let summary = twap_response_fill_summary(&response);
    assert_eq!(summary.oid, Some(77));
    assert_eq!(summary.filled_size, 0.0);
    assert_eq!(summary.avg_price, Some(100.0));
}

#[test]
fn status_unknown_twap_reconciles_to_partial_or_completed_from_account_fills() {
    let now = Instant::now();
    let mut partial = test_twap_order(now, 2.0, false, 2);
    partial.status = TwapStatus::Error;
    partial.child_orders.push(TwapChildOrder {
        index: 1,
        requested_at: now,
        planned_size: 1.0,
        limit_price: 100.0,
        oid: Some(42),
        cloid: Some("0x1234567890abcdef1234567890abcdef".to_string()),
        status: TwapChildStatus::StatusUnknown,
        exchange_summary: "status unknown".to_string(),
        filled_size: 0.0,
        avg_price: None,
        fee: 0.0,
        retry_count: 0,
    });

    partial.reconcile_fills(&[user_fill(42, "1.0", "100")]);
    assert_eq!(partial.status, TwapStatus::CompletedPartial);
    assert_eq!(partial.filled_size, 1.0);
    assert_eq!(partial.remaining_size, 1.0);

    let mut completed = partial.clone();
    completed.target_size = 1.0;
    completed.remaining_size = 1.0;
    completed.filled_size = 0.0;
    completed.status = TwapStatus::Error;
    completed.child_orders[0].filled_size = 0.0;
    completed.child_orders[0].status = TwapChildStatus::StatusUnknown;

    completed.reconcile_fills(&[user_fill(42, "1.0", "100")]);
    assert_eq!(completed.status, TwapStatus::Completed);
    assert_eq!(completed.remaining_size, 0.0);
}

#[test]
fn twap_fill_reconciliation_deduplicates_fills_by_stable_identity() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 2.0, false, 2);
    twap.child_orders.push(TwapChildOrder {
        index: 1,
        requested_at: now,
        planned_size: 1.0,
        limit_price: 100.0,
        oid: Some(42),
        cloid: Some("0x1234567890abcdef1234567890abcdef".to_string()),
        status: TwapChildStatus::AwaitingReconciliation,
        exchange_summary: "filled".to_string(),
        filled_size: 0.0,
        avg_price: None,
        fee: 0.0,
        retry_count: 0,
    });

    let mut first = user_fill(42, "1.0", "100");
    first.tid = Some(123);
    let mut duplicate = user_fill(42, "1.0", "110");
    duplicate.tid = Some(123);

    twap.reconcile_fills(&[first, duplicate]);

    assert_eq!(twap.child_orders[0].filled_size, 1.0);
    assert_eq!(twap.child_orders[0].avg_price, Some(100.0));
    assert_eq!(twap.filled_size, 1.0);
    assert_eq!(twap.remaining_size, 1.0);
}
