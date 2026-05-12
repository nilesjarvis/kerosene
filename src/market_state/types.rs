use crate::account::AssetContext;
use crate::api::{BookLevel, OrderBook};
use crate::config;
use crate::helpers::aggregate_levels;

use std::cell::{Ref, RefCell};
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SymbolSearchSortMode {
    #[default]
    Relevance,
    Volume24h,
    Alphabetical,
    Exchange,
}

impl SymbolSearchSortMode {
    pub(crate) const ALL: [Self; 4] = [
        Self::Relevance,
        Self::Volume24h,
        Self::Alphabetical,
        Self::Exchange,
    ];
}

impl std::fmt::Display for SymbolSearchSortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Relevance => write!(f, "Relevance"),
            Self::Volume24h => write!(f, "24h Vol"),
            Self::Alphabetical => write!(f, "A-Z"),
            Self::Exchange => write!(f, "Exchange"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SymbolSearchMarketFilter {
    #[default]
    All,
    NativePerps,
    Spot,
    Hip3,
    Outcomes,
}

impl SymbolSearchMarketFilter {
    pub(crate) const ALL: [Self; 5] = [
        Self::All,
        Self::NativePerps,
        Self::Spot,
        Self::Hip3,
        Self::Outcomes,
    ];
}

impl std::fmt::Display for SymbolSearchMarketFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All Markets"),
            Self::NativePerps => write!(f, "Native Perps"),
            Self::Spot => write!(f, "Spot"),
            Self::Hip3 => write!(f, "HIP-3"),
            Self::Outcomes => write!(f, "Outcomes"),
        }
    }
}

pub(crate) const SYMBOL_SEARCH_ALL_HIP3_DEXES: &str = "All HIP-3";

pub type LiveWatchlistId = u64;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiveWatchlistRowData {
    pub(crate) sym_key: String,
    pub(crate) display: String,
    pub(crate) mid_px: Option<f64>,
    pub(crate) pct_5m: Option<f64>,
    pub(crate) pct_30m: Option<f64>,
    pub(crate) pct_1h: Option<f64>,
    pub(crate) pct_24h: Option<f64>,
    pub(crate) funding: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LiveWatchlistInstance {
    pub id: LiveWatchlistId,
    pub symbols: Vec<String>,
    pub search_query: String,
    pub sort_column: config::LiveWatchlistSortColumn,
    pub sort_direction: config::SortDirection,
    pub visible_columns: Vec<config::LiveWatchlistColumn>,
    pub(crate) row_cache: Vec<LiveWatchlistRowData>,
}

pub type OrderBookId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderBookDisplayMode {
    #[default]
    DepthList,
    DomLadder,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OrderBookSymbolMode {
    #[default]
    Active,
    Fixed(String),
}

pub struct OrderBookInstance {
    pub id: OrderBookId,
    pub mode: OrderBookSymbolMode,
    pub book: OrderBook,
    pub asset_ctx: Option<AssetContext>,
    pub scroll_id: iced::widget::Id,
    pub tick_size: f64,
    pub settings_open: bool,
    pub search_query: String,
    pub display_mode: OrderBookDisplayMode,
    pub book_loading: bool,
    pub book_error: Option<String>,
    pub show_spread_chart: bool,
    pub spread_history: VecDeque<(Instant, f64)>,
    pub spread_chart_height: f32,
    book_revision: u64,
    aggregated: RefCell<AggregatedDepth>,
}

/// Cached `(price, size, cum_size)` levels for the depth view, keyed by the
/// book revision and tick size they were computed against.
#[derive(Debug, Default, Clone)]
pub struct AggregatedDepth {
    pub bids: Vec<(f64, f64, f64)>,
    pub asks: Vec<(f64, f64, f64)>,
    book_revision: u64,
    tick_bits: u64,
}

impl OrderBookInstance {
    pub fn new(id: OrderBookId, mode: OrderBookSymbolMode, tick_size: f64) -> Self {
        Self {
            id,
            mode,
            book: OrderBook::empty(),
            asset_ctx: None,
            scroll_id: iced::widget::Id::unique(),
            tick_size,
            settings_open: false,
            search_query: String::new(),
            display_mode: OrderBookDisplayMode::DepthList,
            book_loading: false,
            book_error: None,
            show_spread_chart: false,
            spread_history: VecDeque::new(),
            spread_chart_height: 60.0,
            book_revision: 0,
            aggregated: RefCell::new(AggregatedDepth::default()),
        }
    }

    /// Replace the in-memory book snapshot. Bumps the revision counter so any
    /// cached aggregation is invalidated on the next read.
    pub fn set_book(&mut self, book: OrderBook) {
        self.book = book;
        self.book_revision = self.book_revision.wrapping_add(1);
    }

    pub fn set_tick_size(&mut self, tick: f64) {
        self.tick_size = tick;
    }

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
}

pub fn aggregate_with_cumulative(
    levels: &[BookLevel],
    tick: f64,
    is_bid: bool,
) -> Vec<(f64, f64, f64)> {
    let bucketed = aggregate_levels(levels, tick, is_bid);
    let mut cum = 0.0;
    bucketed
        .into_iter()
        .map(|(px, sz)| {
            cum += sz;
            (px, sz, cum)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lvl(px: f64, sz: f64) -> BookLevel {
        BookLevel { px, sz }
    }

    #[test]
    fn aggregate_with_cumulative_returns_empty_for_no_levels() {
        let out = aggregate_with_cumulative(&[], 0.5, false);
        assert!(out.is_empty());
    }

    #[test]
    fn aggregate_with_cumulative_accumulates_asks_from_inside_out() {
        let asks = [lvl(100.0, 1.0), lvl(100.5, 2.0), lvl(101.0, 3.0)];
        let out = aggregate_with_cumulative(&asks, 0.5, false);

        assert_eq!(out.len(), 3);
        assert_eq!(out[0], (100.0, 1.0, 1.0));
        assert_eq!(out[1], (100.5, 2.0, 3.0));
        assert_eq!(out[2], (101.0, 3.0, 6.0));
    }

    #[test]
    fn aggregate_with_cumulative_accumulates_bids_from_inside_out() {
        let bids = [lvl(99.0, 1.5), lvl(98.5, 2.0), lvl(98.0, 0.5)];
        let out = aggregate_with_cumulative(&bids, 0.5, true);

        assert_eq!(out.len(), 3);
        assert_eq!(out[0], (99.0, 1.5, 1.5));
        assert_eq!(out[1], (98.5, 2.0, 3.5));
        assert_eq!(out[2], (98.0, 0.5, 4.0));
    }

    #[test]
    fn aggregate_with_cumulative_groups_sub_tick_levels_into_buckets() {
        // Three sub-tick asks at 99.7 / 99.8 / 99.9 all ceil into the 100.0
        // bucket at tick=0.5; the merged size + cumulative reflect that.
        let asks = [lvl(99.7, 1.0), lvl(99.8, 2.0), lvl(99.9, 4.0)];
        let out = aggregate_with_cumulative(&asks, 0.5, false);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 100.0);
        assert_eq!(out[0].1, 7.0);
        assert_eq!(out[0].2, 7.0);
    }

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
}
