use super::{
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_MAX_AGGREGATE_SLICE_RATE, TWAP_RECONCILIATION_TIMEOUT,
    TwapChildOrder, TwapChildStatus, TwapEvent, TwapEventKind, TwapOrder, TwapOrderInit,
    TwapPauseReason, TwapPendingOp, TwapPendingSlice, TwapStatus, parse_twap_duration_minutes,
    parse_twap_slice_count, quantize_twap_slice_size, twap_aggregate_schedule_has_capacity,
    twap_aggregate_slice_rate, twap_child_cloid, twap_limit_price_for_slice,
    twap_min_quantized_child_notional, twap_order_notional_meets_minimum, twap_required_slice_rate,
    twap_response_fill_summary, twap_target_size_from_quantity, validate_twap_interval,
};
use crate::account::UserFill;
use crate::api::{BookLevel, OrderBook};
use crate::signing::ExchangeResponse;

use std::time::{Duration, Instant};

mod fills;
mod price_gate;
mod sizing;
mod timing;

fn book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
    OrderBook {
        bids: bids
            .iter()
            .map(|(px, sz)| BookLevel { px: *px, sz: *sz })
            .collect(),
        asks: asks
            .iter()
            .map(|(px, sz)| BookLevel { px: *px, sz: *sz })
            .collect(),
    }
}

fn user_fill(oid: u64, size: &str, price: &str) -> UserFill {
    user_fill_for("BTC", "B", oid, size, price)
}

fn user_fill_for(coin: &str, side: &str, oid: u64, size: &str, price: &str) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: price.to_string(),
        sz: size.to_string(),
        side: side.to_string(),
        time: 1,
        hash: None,
        tid: None,
        oid: Some(oid),
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0.01".to_string(),
        fee_token: None,
    }
}

fn test_twap_order(now: Instant, target_size: f64, randomize: bool, slice_count: u32) -> TwapOrder {
    TwapOrder::new(TwapOrderInit {
        id: 1,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "twap-agent-secret".to_string().into(),
        is_buy: true,
        target_size,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        min_price: 90.0,
        max_price: 110.0,
        randomize,
        duration: Duration::from_secs(60),
        slice_count,
        now,
        started_at_ms: 1_000,
    })
}

#[test]
fn twap_order_debug_redacts_agent_key_and_account_address() {
    let mut twap = test_twap_order(Instant::now(), 7.654321, false, 2);
    twap.coin = "SECRET_TWAP_COIN".to_string();
    twap.display_coin = "SECRET TWAP DISPLAY".to_string();
    twap.remaining_size = 6.54321;
    twap.filled_size = 1.11111;
    twap.min_price = 12345.67;
    twap.max_price = 23456.78;
    twap.pending_op = Some(TwapPendingOp::CancelUnexpectedResting {
        oid: Some(424242),
        cloid: Some("pending-cloid-secret".to_string()),
    });
    twap.retry_slice = Some(TwapPendingSlice {
        index: 9,
        planned_size: 2.22222,
        limit_price: 34567.89,
        cloid: "retry-cloid-secret".to_string(),
        retry_count: 3,
    });
    twap.status_check_cloid = Some("status-cloid-secret".to_string());
    twap.status_check_pending_attempt = Some(3);
    twap.unexpected_cancel_pending_attempt = Some(3);
    twap.stop_reason = Some(("stop reason with SECRET_TWAP_COIN".to_string(), true));
    twap.child_orders.push(TwapChildOrder {
        index: 4,
        requested_at: Instant::now(),
        planned_size: 3.33333,
        limit_price: 45678.9,
        oid: Some(777777),
        cloid: Some("child-cloid-secret".to_string()),
        status: TwapChildStatus::Pending,
        exchange_summary: "exchange-summary-secret".to_string(),
        filled_size: 4.44444,
        avg_price: Some(56789.01),
        fee: 5.55555,
        retry_count: 6,
    });

    let rendered = format!("{twap:?}");
    let retry_slice_debug = format!("{:?}", twap.retry_slice.as_ref().expect("retry slice"));
    let cancel_op_debug = format!("{:?}", twap.pending_op.as_ref().expect("pending op"));
    let place_op_debug = format!(
        "{:?}",
        TwapPendingOp::Place(twap.retry_slice.clone().expect("retry slice"))
    );
    let child_debug = format!("{:?}", twap.child_orders.first().expect("child order"));

    assert!(rendered.contains("TwapOrder"));
    assert!(rendered.contains("has_pending_op: true"));
    assert!(rendered.contains("has_retry_slice: true"));
    assert!(rendered.contains("has_status_check_cloid: true"));
    assert!(rendered.contains("has_pending_status_check: true"));
    assert!(rendered.contains("has_pending_unexpected_cancel: true"));
    assert!(rendered.contains("stop_reason_is_error: Some(true)"));
    assert!(rendered.contains("child_orders_count: 1"));
    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("twap-agent-secret"));
    assert!(!rendered.contains("0xabc0000000000000000000000000000000000000"));

    for debug in [
        rendered.as_str(),
        retry_slice_debug.as_str(),
        cancel_op_debug.as_str(),
        place_op_debug.as_str(),
        child_debug.as_str(),
    ] {
        assert!(!debug.contains("SECRET_TWAP_COIN"));
        assert!(!debug.contains("SECRET TWAP DISPLAY"));
        assert!(!debug.contains("pending-cloid-secret"));
        assert!(!debug.contains("retry-cloid-secret"));
        assert!(!debug.contains("status-cloid-secret"));
        assert!(!debug.contains("child-cloid-secret"));
        assert!(!debug.contains("exchange-summary-secret"));
        assert!(!debug.contains("424242"));
        assert!(!debug.contains("777777"));
        assert!(!debug.contains("7.654321"));
        assert!(!debug.contains("6.54321"));
        assert!(!debug.contains("1.11111"));
        assert!(!debug.contains("12345.67"));
        assert!(!debug.contains("23456.78"));
        assert!(!debug.contains("2.22222"));
        assert!(!debug.contains("34567.89"));
        assert!(!debug.contains("3.33333"));
        assert!(!debug.contains("45678.9"));
        assert!(!debug.contains("4.44444"));
        assert!(!debug.contains("56789.01"));
        assert!(!debug.contains("5.55555"));
        assert!(!debug.contains("stop reason with"));
    }
}

#[test]
fn twap_event_debug_redacts_exact_activity_message_without_changing_it() {
    const MESSAGE: &str = "Slice 17 filled 7.123456789 @ 42001.7654321 (oid 9876543210123457)";
    let event = TwapEvent {
        at: Instant::now(),
        kind: TwapEventKind::Filled,
        message: MESSAGE.to_string(),
        is_error: false,
    };

    assert_eq!(event.message, MESSAGE);

    let rendered = format!("{event:?}");
    assert!(rendered.contains("TwapEvent"), "{rendered}");
    assert!(rendered.contains("at:"), "{rendered}");
    assert!(rendered.contains("Filled"), "{rendered}");
    assert!(rendered.contains("is_error: false"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains(MESSAGE), "{rendered}");
}

fn next_slice(twap: &mut TwapOrder, context: &str) -> f64 {
    match twap.next_slice_size() {
        Some(slice) => slice,
        None => panic!("{context}"),
    }
}

fn valid_duration_minutes(value: &str) -> Duration {
    match parse_twap_duration_minutes(value) {
        Some(duration) => duration,
        None => panic!("duration should parse"),
    }
}

fn positive_child_notional(
    target_size: f64,
    slice_count: u32,
    reference_price: f64,
    randomize: bool,
    sz_decimals: u32,
) -> f64 {
    match twap_min_quantized_child_notional(
        target_size,
        slice_count,
        reference_price,
        randomize,
        sz_decimals,
    ) {
        Some(notional) => notional,
        None => panic!("randomized child notional should calculate"),
    }
}

fn exchange_response_from_value(value: serde_json::Value, context: &str) -> ExchangeResponse {
    match serde_json::from_value(value) {
        Ok(response) => response,
        Err(error) => panic!("{context}: {error}"),
    }
}
