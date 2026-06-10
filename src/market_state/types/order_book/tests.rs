use super::*;
use crate::account::AssetContext;
use crate::api::BookLevel;
use crate::market_state::OrderBookSymbolMode;

fn book_at(mid: f64) -> OrderBook {
    let half_spread = mid * 0.01;
    OrderBook {
        bids: vec![BookLevel {
            px: mid - half_spread,
            sz: 1.0,
        }],
        asks: vec![BookLevel {
            px: mid + half_spread,
            sz: 1.0,
        }],
    }
}

fn asset_context_with_impact(bid: &str, ask: &str) -> AssetContext {
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
fn tick_options_basis_holds_within_band_and_tracks_regime_changes() {
    let mut inst = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 0.01);
    inst.set_book_with_source(book_at(100.0), None);
    assert_eq!(inst.tick_options_mid(), 100.0);

    // Mid wobbling around the basis (including across a decade boundary)
    // must not move it — that is the hysteresis the selector relies on.
    inst.set_book_with_source(book_at(95.0), None);
    inst.set_book_with_source(book_at(105.0), None);
    assert_eq!(inst.tick_options_mid(), 100.0);

    // A decisive regime change re-seeds the basis.
    inst.set_book_with_source(book_at(400.0), None);
    assert_eq!(inst.tick_options_mid(), 400.0);

    inst.reset_tick_options_basis();
    inst.set_book_with_source(book_at(50.0), None);
    assert_eq!(inst.tick_options_mid(), 50.0);
}

#[test]
fn tick_options_mid_falls_back_to_book_mid_without_a_basis() {
    let mut inst = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 0.01);
    assert_eq!(inst.tick_options_mid(), 0.0);

    // A book set without a recordable mid leaves the basis unset.
    inst.book = book_at(80.0);
    assert_eq!(inst.tick_options_mid(), 80.0);
}

#[test]
fn visible_best_bid_ask_prefers_book_top_over_impact_prices() {
    let mut inst = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 0.01);
    inst.set_book(OrderBook {
        bids: vec![BookLevel { px: 99.9, sz: 1.0 }],
        asks: vec![BookLevel { px: 100.1, sz: 1.0 }],
    });
    inst.asset_ctx = Some(asset_context_with_impact("99.5", "100.5"));

    // best_bid_ask honours impact prices, but the spread row must agree
    // with the rows it sits between.
    assert_eq!(inst.best_bid_ask(), (Some(99.5), Some(100.5)));
    assert_eq!(inst.visible_best_bid_ask(), (Some(99.9), Some(100.1)));
}

#[test]
fn visible_best_bid_ask_falls_back_to_impact_prices_when_book_is_empty() {
    let mut inst = OrderBookInstance::new(1, OrderBookSymbolMode::Active, 0.01);
    inst.asset_ctx = Some(asset_context_with_impact("99.5", "100.5"));

    assert_eq!(inst.visible_best_bid_ask(), (Some(99.5), Some(100.5)));
}
