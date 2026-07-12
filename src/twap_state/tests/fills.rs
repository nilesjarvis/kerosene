use super::{
    TwapChildOrder, TwapChildStatus, TwapStatus, exchange_response_from_value, test_twap_order,
    twap_response_fill_summary, user_fill, user_fill_for,
};

use std::time::Instant;

#[test]
fn twap_fill_summary_converts_base_token_fees_to_usd() {
    // A spot HYPE/USDC TWAP child: the buy fill's fee arrives in HYPE (base
    // token) and converts at the fill price; the USDC fee passes through.
    let mut base_fee_fill = user_fill(42, "1", "40");
    base_fee_fill.fee = "0.5".to_string();
    base_fee_fill.fee_token = Some("HYPE".to_string());
    let mut usdc_fee_fill = user_fill(42, "1", "40");
    usdc_fee_fill.time = 2;
    usdc_fee_fill.fee = "0.1".to_string();
    usdc_fee_fill.fee_token = Some("USDC".to_string());

    let summary = crate::twap_state::fills::fill_summary_for_order(
        &[base_fee_fill, usdc_fee_fill],
        42,
        "BTC",
        true,
    )
    .expect("fills for oid");

    // 0.5 HYPE * $40 + $0.1 = $20.1
    assert!((summary.fee - 20.1).abs() < 1e-9);
}

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

    assert!(!response.is_fully_filled());
    assert!(response.reports_filled());
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
fn twap_fill_reconciliation_requires_matching_coin_and_side() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 2.0, false, 2);
    twap.coin = "flx:BTC".to_string();
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

    twap.reconcile_fills(&[
        user_fill_for("BTC", "B", 42, "1.0", "100"),
        user_fill_for("flx:BTC", "A", 42, "1.0", "100"),
        user_fill_for("flx:BTC", "B", 42, "0.25", "100"),
    ]);

    assert_eq!(twap.child_orders[0].filled_size, 0.25);
    assert_eq!(twap.filled_size, 0.25);
    assert_eq!(twap.remaining_size, 1.75);
}

#[test]
fn twap_fill_reconciliation_distinguishes_native_and_hip3_same_oid() {
    let now = Instant::now();
    let mut native = test_twap_order(now, 1.0, false, 2);
    native.child_orders.push(TwapChildOrder {
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
    native.reconcile_fills(&[user_fill_for("flx:BTC", "B", 42, "1.0", "100")]);
    assert_eq!(native.filled_size, 0.0);
    assert_eq!(native.remaining_size, 1.0);

    let mut hip3 = test_twap_order(now, 1.0, false, 2);
    hip3.coin = "flx:BTC".to_string();
    hip3.child_orders.push(TwapChildOrder {
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
    hip3.reconcile_fills(&[
        user_fill_for("BTC", "B", 42, "1.0", "100"),
        user_fill_for("flx:BTC", "B", 42, "0.4", "100"),
    ]);
    assert_eq!(hip3.filled_size, 0.4);
    assert_eq!(hip3.remaining_size, 0.6);
}

#[test]
fn twap_sell_fill_reconciliation_requires_ask_side() {
    let now = Instant::now();
    let mut twap = test_twap_order(now, 1.0, false, 2);
    twap.is_buy = false;
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

    twap.reconcile_fills(&[
        user_fill_for("BTC", "B", 42, "1.0", "100"),
        user_fill_for("BTC", "A", 42, "0.25", "100"),
    ]);

    assert_eq!(twap.child_orders[0].filled_size, 0.25);
    assert_eq!(twap.filled_size, 0.25);
    assert_eq!(twap.remaining_size, 0.75);
}

#[test]
fn twap_fill_reconciliation_requires_exact_spot_and_outcome_symbols() {
    for (expected_coin, mismatched_coin) in [("@107", "BTC"), ("#950", "@107")] {
        let now = Instant::now();
        let mut twap = test_twap_order(now, 1.0, false, 2);
        twap.coin = expected_coin.to_string();
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

        twap.reconcile_fills(&[user_fill_for(mismatched_coin, "B", 42, "1.0", "100")]);

        assert_eq!(twap.child_orders[0].filled_size, 0.0, "{expected_coin}");
        assert_eq!(twap.filled_size, 0.0, "{expected_coin}");
        assert_eq!(twap.remaining_size, 1.0, "{expected_coin}");
    }
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
