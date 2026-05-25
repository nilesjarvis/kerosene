use super::{AggregatedDepth, OrderBookInstance, aggregate_with_cumulative};
use crate::market_state::dom_ladder::{DomLadderCacheKey, DomLadderRows, build_dom_ladder_rows};

use std::cell::Ref;

// ---------------------------------------------------------------------------
// Order Book Projection Caches
// ---------------------------------------------------------------------------

impl OrderBookInstance {
    /// Cached aggregation of the current book at `tick`, with cumulative depth
    /// attached per level. Recomputed when the book revision or tick size
    /// differs from what's stored in the cache.
    pub fn aggregated_depth(&self, tick: f64) -> Ref<'_, AggregatedDepth> {
        let needs_refresh = {
            let cache = self.aggregated.borrow();
            cache.book_revision != self.book_revision || cache.tick_bits != tick.to_bits()
        };
        if needs_refresh {
            let mut cache = self.aggregated.borrow_mut();
            cache.asks = aggregate_with_cumulative(&self.book.asks, tick, false);
            cache.bids = aggregate_with_cumulative(&self.book.bids, tick, true);
            cache.book_revision = self.book_revision;
            cache.tick_bits = tick.to_bits();
        }
        self.aggregated.borrow()
    }

    /// Cached DOM-ladder rows for the current book at `tick` with
    /// `side_rows` per side. Recomputed only when the book revision,
    /// tick size, or row count differs from the cached entry. Cuts the
    /// DOM view's per-paint work from full aggregation + map allocation
    /// + row vector build down to a borrow.
    pub fn dom_ladder_rows(&self, tick: f64, side_rows: usize) -> Ref<'_, DomLadderRows> {
        let key = DomLadderCacheKey {
            book_revision: self.book_revision,
            tick_bits: tick.to_bits(),
            side_rows,
        };
        let needs_refresh = {
            let cache = self.dom_ladder.borrow();
            !cache.populated || cache.key != key
        };
        if needs_refresh {
            let mut cache = self.dom_ladder.borrow_mut();
            cache.rows = build_dom_ladder_rows(&self.book, tick, side_rows);
            cache.key = key;
            cache.populated = true;
        }
        Ref::map(self.dom_ladder.borrow(), |cache| &cache.rows)
    }
}
