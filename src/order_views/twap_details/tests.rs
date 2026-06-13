use super::formatting::{
    child_id_text, twap_next_retry_text, twap_pause_text, twap_status_check_text,
    weighted_average_fill_price,
};
use crate::twap_state::{
    TwapChildOrder, TwapChildStatus, TwapOrder, TwapOrderInit, TwapPauseReason,
};

use std::time::{Duration, Instant};

#[test]
fn twap_detail_average_uses_valid_filled_children_only() {
    let now = Instant::now();
    let mut twap = twap_order(now);
    twap.child_orders = vec![
        child(now, 1, 1.0, Some(100.0)),
        child(now, 2, 3.0, Some(110.0)),
        child(now, 3, 0.0, Some(1.0)),
        child(now, 4, 2.0, Some(f64::INFINITY)),
        child(now, 5, 2.0, None),
        child(now, 6, f64::INFINITY, Some(999.0)),
    ];

    assert_eq!(weighted_average_fill_price(&twap), Some(107.5));

    for child in &mut twap.child_orders {
        child.filled_size = 0.0;
    }
    assert_eq!(weighted_average_fill_price(&twap), None);
}

#[test]
fn twap_detail_identifiers_and_status_text_match_display_contract() {
    let now = Instant::now();
    let mut child = child(now, 1, 0.0, None);
    assert_eq!(child_id_text(&child), "-");

    child.oid = Some(12345);
    assert_eq!(child_id_text(&child), "#12345");

    child.cloid = Some("abcdefghijklmno".to_string());
    assert_eq!(child_id_text(&child), "#12345 abcdefghij...");

    child.oid = None;
    assert_eq!(child_id_text(&child), "abcdefghij...");

    let mut twap = twap_order(now);
    assert_eq!(twap_pause_text(&twap), "-");
    assert_eq!(twap_status_check_text(&twap), "-");

    twap.pause_reason = Some(TwapPauseReason::RateLimited);
    twap.paused_until = Some(now + Duration::from_secs(12));
    twap.status_check_cloid = Some("1234567890abcdef".to_string());
    twap.status_check_retries = 2;
    assert_eq!(twap_pause_text(&twap), "Rate limited");
    assert_eq!(twap_next_retry_text(&twap, now), "12s");
    assert_eq!(
        twap_next_retry_text(&twap, now + Duration::from_secs(12)),
        "Now"
    );
    assert_eq!(twap_status_check_text(&twap), "1234567890... (2)");
}

fn twap_order(now: Instant) -> TwapOrder {
    TwapOrder::new(TwapOrderInit {
        id: 1,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: "0xabc".to_string(),
        agent_key: "key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        min_price: 90.0,
        max_price: 110.0,
        randomize: false,
        duration: Duration::from_secs(60),
        slice_count: 4,
        now,
        started_at_ms: 1_000,
    })
}

fn child(now: Instant, index: u32, filled_size: f64, avg_price: Option<f64>) -> TwapChildOrder {
    TwapChildOrder {
        index,
        requested_at: now,
        planned_size: 1.0,
        limit_price: 100.0,
        oid: None,
        cloid: None,
        status: TwapChildStatus::Pending,
        exchange_summary: String::new(),
        filled_size,
        avg_price,
        fee: 0.0,
        retry_count: 0,
    }
}
