use super::super::{OrderBookInstance, OrderBookSymbolMode};
use super::book_at_mid;
use crate::account::AssetContext;
use crate::api::{BookLevel, OrderBook};

use std::time::{Duration, Instant};

fn asset_ctx_with_impact(bid: &str, ask: &str) -> AssetContext {
    AssetContext {
        funding: None,
        open_interest: None,
        oracle_px: None,
        mark_px: None,
        mid_px: None,
        prev_day_px: None,
        day_ntl_vlm: None,
        day_base_vlm: None,
        impact_pxs: Some(vec![bid.to_string(), ask.to_string()]),
    }
}

#[test]
fn record_spread_sample_uses_visible_book_top() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.set_book(book_at_mid(100.0));
    // Impact prices would give a wider spread; the chart must mirror the row
    // and use the book top instead.
    inst.asset_ctx = Some(asset_ctx_with_impact("90", "110"));

    inst.record_spread_sample(now);

    assert_eq!(inst.spread_history.front().copied(), Some((now, 1.0)));
}

#[test]
fn record_spread_sample_falls_back_to_impact_when_book_is_empty() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.asset_ctx = Some(asset_ctx_with_impact("90", "110"));

    inst.record_spread_sample(now);

    assert_eq!(inst.spread_history.front().copied(), Some((now, 20.0)));
}

#[test]
fn record_spread_sample_skips_one_sided_book() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.set_book(OrderBook {
        bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
        asks: vec![],
    });

    inst.record_spread_sample(now);

    assert!(inst.spread_history.is_empty());
}

#[test]
fn record_spread_sample_throttles_rapid_samples() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.set_book(book_at_mid(100.0));

    inst.record_spread_sample(now);
    // A burst of book updates within the throttle window must not flood the
    // history — only the first lands.
    inst.record_spread_sample(now + Duration::from_millis(100));
    inst.record_spread_sample(now + Duration::from_millis(500));
    assert_eq!(inst.spread_history.len(), 1);

    // Once the minimum interval elapses, the next sample is recorded.
    inst.set_book(OrderBook {
        bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
        asks: vec![BookLevel { px: 101.0, sz: 1.0 }],
    });
    inst.record_spread_sample(now + Duration::from_secs(1));
    assert_eq!(inst.spread_history.len(), 2);
    assert_eq!(
        inst.spread_history.front().map(|(_, spread)| *spread),
        Some(2.0)
    );
}

#[test]
fn record_spread_sample_trims_samples_older_than_window() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.set_book(book_at_mid(100.0));

    // A sample outside the 300-second window is dropped on the next record.
    inst.spread_history
        .push_back((now - Duration::from_secs(301), 5.0));
    inst.record_spread_sample(now);

    assert!(inst.spread_history.len() <= 1);
    assert_eq!(inst.spread_history.front().copied(), Some((now, 1.0)));
}

#[test]
fn clear_asset_context_preserves_history_while_full_reset_drops_it() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.set_book(book_at_mid(100.0));
    inst.record_spread_sample(now);
    inst.record_mid_price_sample(now);
    assert!(!inst.spread_history.is_empty());
    assert!(!inst.mid_price_history.is_empty());

    // An asset-context lag/staleness must not wipe book-derived history.
    inst.clear_asset_context();
    assert!(inst.asset_ctx.is_none());
    assert!(!inst.spread_history.is_empty());
    assert!(!inst.mid_price_history.is_empty());

    // A symbol change / book reset drops everything.
    inst.clear_asset_context_and_price_history();
    assert!(inst.spread_history.is_empty());
    assert!(inst.mid_price_history.is_empty());
}
