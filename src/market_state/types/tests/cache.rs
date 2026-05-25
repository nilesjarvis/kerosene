use super::super::{OrderBookInstance, OrderBookSymbolMode};
use super::lvl;
use crate::api::OrderBook;

#[test]
fn aggregated_depth_cache_serves_repeated_reads_without_recomputing() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 0.5);
    inst.set_book(OrderBook {
        bids: vec![lvl(99.0, 1.0), lvl(98.5, 2.0)],
        asks: vec![lvl(100.0, 1.0), lvl(100.5, 2.0)],
    });

    let revision_before = inst.book_revision;
    let first = inst.aggregated_depth(0.5).clone();
    let second = inst.aggregated_depth(0.5).clone();

    assert_eq!(first.book_revision, revision_before);
    assert_eq!(second.book_revision, revision_before);
    assert_eq!(first.asks, second.asks);
    assert_eq!(first.bids, second.bids);
}

#[test]
fn aggregated_depth_cache_invalidates_when_book_changes() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 0.5);
    inst.set_book(OrderBook {
        bids: vec![lvl(99.0, 1.0)],
        asks: vec![lvl(100.0, 1.0)],
    });
    let first_rev = {
        let depth = inst.aggregated_depth(0.5);
        depth.book_revision
    };

    inst.set_book(OrderBook {
        bids: vec![lvl(99.0, 5.0)],
        asks: vec![lvl(100.0, 5.0)],
    });
    let second = inst.aggregated_depth(0.5);

    assert!(second.book_revision != first_rev);
    assert_eq!(second.asks[0], (100.0, 5.0, 5.0));
    assert_eq!(second.bids[0], (99.0, 5.0, 5.0));
}

#[test]
fn aggregated_depth_cache_invalidates_when_tick_changes() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 0.5);
    inst.set_book(OrderBook {
        // At tick=0.5 the two asks live in distinct buckets (100.5, 101.0).
        // At tick=1.0 they both ceil into the 101.0 bucket.
        bids: vec![lvl(99.0, 1.0)],
        asks: vec![lvl(100.5, 1.0), lvl(101.0, 2.0)],
    });

    let fine_tick_bits = inst.aggregated_depth(0.5).tick_bits;
    let fine_levels = inst.aggregated_depth(0.5).asks.len();
    let coarse_tick_bits = inst.aggregated_depth(1.0).tick_bits;
    let coarse_levels = inst.aggregated_depth(1.0).asks.len();

    assert_ne!(fine_tick_bits, coarse_tick_bits);
    assert_eq!(fine_levels, 2);
    assert_eq!(coarse_levels, 1);
}

#[test]
fn order_book_source_precision_blocks_fake_finer_rendering() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
    inst.set_book_with_source(
        OrderBook {
            bids: vec![lvl(100.0, 1.0)],
            asks: vec![lvl(105.0, 1.0)],
        },
        Some(5.0),
    );

    assert!(inst.can_render_book_at_tick(5.0));
    assert!(inst.can_render_book_at_tick(10.0));
    assert!(!inst.can_render_book_at_tick(1.0));
}

#[test]
fn unknown_source_precision_is_renderable() {
    let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
    inst.set_book(OrderBook {
        bids: vec![lvl(100.0, 1.0)],
        asks: vec![lvl(101.0, 1.0)],
    });

    assert!(inst.can_render_book_at_tick(0.1));
}
