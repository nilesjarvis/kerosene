use super::*;
use crate::twap_state::{TwapChildOrder, TwapChildStatus, TwapOrder, TwapOrderInit};

use std::time::{Duration, Instant};

#[test]
fn twap_weighted_average_uses_only_positive_finite_sizes_and_prices() {
    let now = Instant::now();
    let mut twap = TwapOrder::new(TwapOrderInit {
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
    });
    twap.child_orders = vec![
        child(now, 1.0, Some(100.0)),
        child(now, 3.0, Some(110.0)),
        child(now, 0.0, Some(1.0)),
        child(now, f64::INFINITY, Some(999.0)),
        child(now, 2.0, Some(f64::INFINITY)),
        child(now, 2.0, None),
    ];

    assert_eq!(twap_weighted_average_fill_price(&twap), Some(107.5));

    for child in &mut twap.child_orders {
        child.filled_size = 0.0;
    }
    assert_eq!(twap_weighted_average_fill_price(&twap), None);
}

fn child(now: Instant, filled_size: f64, avg_price: Option<f64>) -> TwapChildOrder {
    TwapChildOrder {
        index: 1,
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
