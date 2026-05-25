use crate::api::{BookLevel, OrderBook};
use crate::market_state::build_dom_ladder_rows;

fn level(px: f64, sz: f64) -> BookLevel {
    BookLevel { px, sz }
}

#[test]
fn dom_ladder_builds_contiguous_ask_and_bid_rows() {
    let book = OrderBook {
        bids: vec![level(99.0, 2.0), level(97.0, 3.0)],
        asks: vec![level(101.0, 4.0), level(103.0, 5.0)],
    };

    let rows = build_dom_ladder_rows(&book, 1.0, 4);

    assert_eq!(rows.asks.len(), 4);
    assert_eq!(rows.bids.len(), 4);
    assert_eq!(
        rows.asks.iter().map(|row| row.price).collect::<Vec<_>>(),
        vec![104.0, 103.0, 102.0, 101.0]
    );
    assert_eq!(
        rows.bids.iter().map(|row| row.price).collect::<Vec<_>>(),
        vec![99.0, 98.0, 97.0, 96.0]
    );
}

#[test]
fn dom_ladder_puts_sizes_and_cumulative_totals_on_the_correct_side() {
    let book = OrderBook {
        bids: vec![level(99.0, 2.0), level(98.0, 3.0)],
        asks: vec![level(101.0, 4.0), level(102.0, 5.0)],
    };

    let rows = build_dom_ladder_rows(&book, 1.0, 2);

    assert_eq!(rows.asks[0].price, 102.0);
    assert_eq!(rows.asks[0].ask_size, Some(5.0));
    assert_eq!(rows.asks[0].ask_cumulative, Some(9.0));
    assert_eq!(rows.asks[1].price, 101.0);
    assert_eq!(rows.asks[1].ask_size, Some(4.0));
    assert_eq!(rows.asks[1].ask_cumulative, Some(4.0));
    assert_eq!(rows.asks[1].bid_size, None);

    assert_eq!(rows.bids[0].price, 99.0);
    assert_eq!(rows.bids[0].bid_size, Some(2.0));
    assert_eq!(rows.bids[0].bid_cumulative, Some(2.0));
    assert_eq!(rows.bids[1].price, 98.0);
    assert_eq!(rows.bids[1].bid_size, Some(3.0));
    assert_eq!(rows.bids[1].bid_cumulative, Some(5.0));
    assert_eq!(rows.bids[0].ask_size, None);
}

#[test]
fn dom_ladder_handles_empty_or_invalid_inputs_without_panics() {
    let empty = build_dom_ladder_rows(&OrderBook::empty(), 1.0, 10);
    assert!(empty.asks.is_empty());
    assert!(empty.bids.is_empty());

    let invalid_tick = build_dom_ladder_rows(&OrderBook::empty(), 0.0, 10);
    assert!(invalid_tick.asks.is_empty());
    assert!(invalid_tick.bids.is_empty());
}
