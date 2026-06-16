use super::{
    MIN_EXCHANGE_ORDER_NOTIONAL_USD, TWAP_MAX_AGGREGATE_SLICE_RATE, TWAP_RECONCILIATION_TIMEOUT,
    TwapChildOrder, TwapChildStatus, TwapOrder, TwapOrderInit, TwapPauseReason, TwapStatus,
    parse_twap_duration_minutes, parse_twap_slice_count, quantize_twap_slice_size,
    twap_aggregate_schedule_has_capacity, twap_aggregate_slice_rate, twap_child_cloid,
    twap_limit_price_for_slice, twap_min_quantized_child_notional,
    twap_order_notional_meets_minimum, twap_required_slice_rate, twap_response_fill_summary,
    twap_target_size_from_quantity, validate_twap_interval,
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
    }
}

fn test_twap_order(now: Instant, target_size: f64, randomize: bool, slice_count: u32) -> TwapOrder {
    TwapOrder::new(TwapOrderInit {
        id: 1,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: "0xabc".to_string(),
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
fn twap_order_debug_redacts_agent_key() {
    let twap = test_twap_order(Instant::now(), 1.0, false, 2);

    let rendered = format!("{twap:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("twap-agent-secret"));
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
