use super::super::{OrderBookInstance, OrderBookSymbolMode};
use super::book_at_mid;
use crate::account::AssetContext;
use crate::market_state::MARKET_ASSET_CONTEXT_MAX_AGE_MS;

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
fn stale_asset_context_expiry_clears_derived_context_without_dropping_book_rows() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    let now = Instant::now();
    inst.set_book(book_at_mid(100.0));
    inst.asset_ctx = Some(asset_ctx_with_impact("90", "110"));
    inst.asset_ctx_updated_at = Some(now);
    inst.spread_history.push_back((now, 20.0));
    inst.record_mid_price_sample(now);

    assert!(!inst.expire_asset_context_if_stale(
        now + Duration::from_millis(MARKET_ASSET_CONTEXT_MAX_AGE_MS)
    ));
    assert_eq!(inst.best_bid_ask(), (Some(90.0), Some(110.0)));
    assert!(!inst.spread_history.is_empty());
    assert!(!inst.mid_price_history.is_empty());

    assert!(inst.expire_asset_context_if_stale(
        now + Duration::from_millis(MARKET_ASSET_CONTEXT_MAX_AGE_MS + 1)
    ));
    assert!(inst.asset_ctx.is_none());
    assert!(inst.asset_ctx_updated_at.is_none());
    // Asset-context expiry drops the impact context but leaves the
    // book-derived spread/mid history intact — the live L2 book still owns
    // those, and a stale asset context must not blank the spread chart.
    assert!(!inst.spread_history.is_empty());
    assert!(!inst.mid_price_history.is_empty());
    assert_eq!(inst.best_bid_ask(), (Some(99.5), Some(100.5)));
    assert_eq!(inst.book.bids.len(), 1);
    assert_eq!(inst.book.asks.len(), 1);
}
