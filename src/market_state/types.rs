use crate::account::AssetContext;
use crate::api::{BookLevel, OrderBook};
use crate::config;
use crate::helpers::{aggregate_levels, tick_sizes_match};

use std::cell::{Ref, RefCell};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

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

    pub(crate) fn from_config_str(value: &str) -> Self {
        match value {
            "24h_volume" => Self::Volume24h,
            "alphabetical" => Self::Alphabetical,
            "exchange" => Self::Exchange,
            _ => Self::Relevance,
        }
    }

    pub(crate) fn config_value(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Volume24h => "24h_volume",
            Self::Alphabetical => "alphabetical",
            Self::Exchange => "exchange",
        }
    }
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

#[cfg(test)]
mod symbol_search_tests {
    use super::*;

    #[test]
    fn symbol_search_sort_mode_config_values_round_trip() {
        for mode in SymbolSearchSortMode::ALL {
            assert_eq!(
                SymbolSearchSortMode::from_config_str(mode.config_value()),
                mode
            );
        }
        assert_eq!(
            SymbolSearchSortMode::from_config_str("unknown"),
            SymbolSearchSortMode::Relevance
        );
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

const SHORT_TERM_PRICE_MOVE_WINDOW: Duration = Duration::from_secs(3);
const SHORT_TERM_PRICE_HISTORY_WINDOW: Duration = Duration::from_secs(10);
const SHORT_TERM_PRICE_HISTORY_LIMIT: usize = 2_048;

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
    pub center_on_mid: bool,
    pub show_spread_chart: bool,
    pub spread_history: VecDeque<(Instant, f64)>,
    pub spread_chart_height: f32,
    mid_price_history: VecDeque<(Instant, f64)>,
    book_source_tick_size: Option<f64>,
    pending_book_sigfigs: Option<(Option<u8>, Option<u8>)>,
    book_revision: u64,
    aggregated: RefCell<AggregatedDepth>,
    dom_ladder: RefCell<super::dom_ladder::DomLadderCache>,
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
            center_on_mid: false,
            show_spread_chart: false,
            spread_history: VecDeque::new(),
            spread_chart_height: 60.0,
            mid_price_history: VecDeque::new(),
            book_source_tick_size: None,
            pending_book_sigfigs: None,
            book_revision: 0,
            aggregated: RefCell::new(AggregatedDepth::default()),
            dom_ladder: RefCell::new(super::dom_ladder::DomLadderCache::default()),
        }
    }

    /// Replace the in-memory book snapshot. Bumps the revision counter so any
    /// cached aggregation is invalidated on the next read.
    pub fn set_book(&mut self, book: OrderBook) {
        self.set_book_with_source(book, None);
    }

    pub fn set_book_with_source(&mut self, book: OrderBook, source_tick_size: Option<f64>) {
        self.book = book;
        self.book_source_tick_size = source_tick_size;
        self.book_revision = self.book_revision.wrapping_add(1);
    }

    pub fn book_source_tick_size(&self) -> Option<f64> {
        self.book_source_tick_size
    }

    pub fn pending_book_sigfigs(&self) -> Option<(Option<u8>, Option<u8>)> {
        self.pending_book_sigfigs
    }

    pub fn mark_book_request(&mut self, sigfigs: (Option<u8>, Option<u8>)) {
        self.pending_book_sigfigs = Some(sigfigs);
    }

    pub fn clear_matching_book_request(&mut self, sigfigs: (Option<u8>, Option<u8>)) {
        if self.pending_book_sigfigs == Some(sigfigs) {
            self.pending_book_sigfigs = None;
        }
    }

    pub fn clear_book_request(&mut self) {
        self.pending_book_sigfigs = None;
    }

    pub fn apply_book_update_preserving_scope(
        &mut self,
        incoming: OrderBook,
        incoming_source_tick_size: Option<f64>,
    ) {
        if self.should_merge_finer_book(incoming_source_tick_size) {
            if let Some(merged) = merge_books_preserving_scope(&self.book, &incoming) {
                self.book = merged;
                self.book_revision = self.book_revision.wrapping_add(1);
            }
        } else {
            self.set_book_with_source(incoming, incoming_source_tick_size);
        }
    }

    pub fn set_tick_size(&mut self, tick: f64) {
        self.tick_size = tick;
    }

    pub fn best_bid_ask(&self) -> (Option<f64>, Option<f64>) {
        let mut true_best_bid = self.book.bids.first().map(|level| level.px);
        let mut true_best_ask = self.book.asks.first().map(|level| level.px);

        if let Some(ctx) = &self.asset_ctx
            && let Some(impact) = &ctx.impact_pxs
            && impact.len() >= 2
            && let (Ok(best_bid), Ok(best_ask)) =
                (impact[0].parse::<f64>(), impact[1].parse::<f64>())
        {
            true_best_bid = Some(best_bid);
            true_best_ask = Some(best_ask);
        }

        (positive_finite(true_best_bid), positive_finite(true_best_ask))
    }

    pub fn current_mid_price(&self) -> Option<f64> {
        let (best_bid, best_ask) = self.best_bid_ask();
        let mid = match (best_bid, best_ask) {
            (Some(best_bid), Some(best_ask)) => (best_bid + best_ask) / 2.0,
            (Some(best_bid), None) => best_bid,
            (None, Some(best_ask)) => best_ask,
            (None, None) => return None,
        };

        positive_finite(Some(mid))
    }

    pub fn record_mid_price_sample(&mut self, now: Instant) {
        let Some(mid) = self.current_mid_price() else {
            return;
        };

        if let Some((latest_time, latest_mid)) = self.mid_price_history.front_mut()
            && mid_prices_match(*latest_mid, mid)
        {
            *latest_time = now;
            self.trim_mid_price_history(now);
            return;
        }

        self.mid_price_history.push_front((now, mid));
        self.trim_mid_price_history(now);
    }

    pub fn clear_mid_price_history(&mut self) {
        self.mid_price_history.clear();
    }

    pub fn short_term_price_move(&self) -> Option<f64> {
        let (latest_time, latest_price) = self.mid_price_history.front().copied()?;
        let cutoff = latest_time
            .checked_sub(SHORT_TERM_PRICE_MOVE_WINDOW)
            .unwrap_or(latest_time);

        let mut reference = None;
        for (time, price) in &self.mid_price_history {
            if *time < cutoff {
                break;
            }
            reference = Some((*time, *price));
        }

        let (reference_time, reference_price) = reference?;
        if reference_time == latest_time {
            return None;
        }

        Some(latest_price - reference_price)
    }

    pub fn can_render_book_at_tick(&self, tick: f64) -> bool {
        self.book_source_tick_size
            .is_none_or(|source_tick| source_tick <= tick || tick_sizes_match(source_tick, tick))
    }

    fn should_merge_finer_book(&self, incoming_source_tick_size: Option<f64>) -> bool {
        let Some(current_source_tick) = self.book_source_tick_size else {
            return false;
        };
        let Some(incoming_source_tick) = incoming_source_tick_size else {
            return false;
        };

        incoming_source_tick < current_source_tick
            && self.can_render_book_at_tick(self.tick_size)
            && (!self.book.bids.is_empty() || !self.book.asks.is_empty())
    }

    fn trim_mid_price_history(&mut self, now: Instant) {
        let cutoff = now
            .checked_sub(SHORT_TERM_PRICE_HISTORY_WINDOW)
            .unwrap_or(now);

        while self.mid_price_history.len() > SHORT_TERM_PRICE_HISTORY_LIMIT {
            self.mid_price_history.pop_back();
        }
        while self
            .mid_price_history
            .back()
            .is_some_and(|(time, _)| *time < cutoff)
        {
            self.mid_price_history.pop_back();
        }
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

    /// Cached DOM-ladder rows for the current book at `tick` with
    /// `side_rows` per side. Recomputed only when the book revision,
    /// tick size, or row count differs from the cached entry. Cuts the
    /// DOM view's per-paint work from full aggregation + map allocation
    /// + row vector build down to a borrow.
    pub fn dom_ladder_rows(
        &self,
        tick: f64,
        side_rows: usize,
    ) -> Ref<'_, super::dom_ladder::DomLadderRows> {
        let key = super::dom_ladder::DomLadderCacheKey {
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
            cache.rows = super::dom_ladder::build_dom_ladder_rows(&self.book, tick, side_rows);
            cache.key = key;
            cache.populated = true;
        }
        Ref::map(self.dom_ladder.borrow(), |cache| &cache.rows)
    }
}

fn positive_finite(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn mid_prices_match(left: f64, right: f64) -> bool {
    let tolerance = f64::EPSILON * 8.0 * left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= tolerance
}

fn merge_books_preserving_scope(current: &OrderBook, incoming: &OrderBook) -> Option<OrderBook> {
    if incoming.bids.is_empty() || incoming.asks.is_empty() {
        return None;
    }

    Some(OrderBook {
        bids: merge_book_side_preserving_scope(&current.bids, &incoming.bids, true),
        asks: merge_book_side_preserving_scope(&current.asks, &incoming.asks, false),
    })
}

fn merge_book_side_preserving_scope(
    current: &[BookLevel],
    incoming: &[BookLevel],
    is_bid: bool,
) -> Vec<BookLevel> {
    if current.is_empty() {
        return incoming.to_vec();
    }

    let min_incoming = incoming
        .iter()
        .map(|level| level.px)
        .fold(f64::INFINITY, f64::min);
    let max_incoming = incoming
        .iter()
        .map(|level| level.px)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut merged = Vec::with_capacity(current.len() + incoming.len());
    merged.extend(incoming.iter().cloned());
    merged.extend(
        current
            .iter()
            .filter(|level| {
                if is_bid {
                    level.px < min_incoming
                } else {
                    level.px > max_incoming
                }
            })
            .cloned(),
    );

    if is_bid {
        merged.sort_by(|a, b| b.px.partial_cmp(&a.px).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        merged.sort_by(|a, b| a.px.partial_cmp(&b.px).unwrap_or(std::cmp::Ordering::Equal));
    }
    merged
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

    fn book_at_mid(mid: f64) -> OrderBook {
        OrderBook {
            bids: vec![lvl(mid - 0.5, 1.0)],
            asks: vec![lvl(mid + 0.5, 1.0)],
        }
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

    #[test]
    fn short_term_price_move_tracks_recent_mid_delta() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
        let now = Instant::now();

        inst.set_book(book_at_mid(100.0));
        inst.record_mid_price_sample(now);
        inst.set_book(book_at_mid(101.25));
        inst.record_mid_price_sample(now + Duration::from_secs(3));

        assert_eq!(inst.short_term_price_move(), Some(1.25));
    }

    #[test]
    fn short_term_price_move_uses_oldest_sample_inside_three_second_window() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
        let now = Instant::now();

        inst.set_book(book_at_mid(100.0));
        inst.record_mid_price_sample(now);
        inst.set_book(book_at_mid(101.0));
        inst.record_mid_price_sample(now + Duration::from_secs(2));
        inst.set_book(book_at_mid(102.5));
        inst.record_mid_price_sample(now + Duration::from_secs(5));

        assert_eq!(inst.short_term_price_move(), Some(1.5));
    }

    #[test]
    fn unchanged_mid_samples_update_time_without_growing_history() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
        let now = Instant::now();
        let later = now + Duration::from_secs(3);

        inst.set_book(book_at_mid(100.0));
        inst.record_mid_price_sample(now);
        inst.record_mid_price_sample(later);

        assert_eq!(inst.mid_price_history.len(), 1);
        assert_eq!(inst.mid_price_history.front().copied(), Some((later, 100.0)));
        assert_eq!(inst.short_term_price_move(), None);
    }

    #[test]
    fn clearing_mid_price_history_removes_short_term_move() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 1.0);
        let now = Instant::now();

        inst.set_book(book_at_mid(100.0));
        inst.record_mid_price_sample(now);
        inst.set_book(book_at_mid(99.5));
        inst.record_mid_price_sample(now + Duration::from_secs(2));

        inst.clear_mid_price_history();

        assert_eq!(inst.short_term_price_move(), None);
    }

    #[test]
    fn finer_live_update_preserves_coarse_snapshot_scope() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(100.0, 10.0), lvl(95.0, 20.0), lvl(90.0, 30.0)],
                asks: vec![lvl(105.0, 10.0), lvl(110.0, 20.0), lvl(115.0, 30.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(100.0, 1.0), lvl(99.0, 2.0)],
                asks: vec![lvl(101.0, 1.0), lvl(102.0, 2.0)],
            },
            Some(1.0),
        );

        assert_eq!(
            inst.book
                .bids
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![100.0, 99.0, 95.0, 90.0]
        );
        assert_eq!(
            inst.book
                .asks
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![101.0, 102.0, 105.0, 110.0, 115.0]
        );
        assert_eq!(inst.book.bids[0].sz, 1.0);
        assert_eq!(inst.book.asks[0].sz, 1.0);
    }

    #[test]
    fn finer_bid_update_drops_stale_bids_above_fresh_scope() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(105.0, 10.0), lvl(100.0, 20.0), lvl(95.0, 30.0)],
                asks: vec![lvl(110.0, 10.0), lvl(115.0, 20.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(100.0, 1.0), lvl(99.0, 2.0)],
                asks: vec![lvl(110.0, 1.0), lvl(111.0, 2.0)],
            },
            Some(1.0),
        );

        assert_eq!(
            inst.book
                .bids
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![100.0, 99.0, 95.0]
        );
        assert_eq!(inst.book.bids[0].sz, 1.0);
        assert!(!inst.book.bids.iter().any(|level| level.px == 105.0));
    }

    #[test]
    fn finer_ask_update_drops_stale_asks_below_fresh_scope() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(95.0, 10.0), lvl(90.0, 20.0)],
                asks: vec![lvl(101.0, 10.0), lvl(105.0, 20.0), lvl(115.0, 30.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(95.0, 1.0), lvl(94.0, 2.0)],
                asks: vec![lvl(110.0, 1.0), lvl(111.0, 2.0)],
            },
            Some(1.0),
        );

        assert_eq!(
            inst.book
                .asks
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![110.0, 111.0, 115.0]
        );
        assert_eq!(inst.book.asks[0].sz, 1.0);
        assert!(!inst.book.asks.iter().any(|level| level.px == 101.0));
        assert!(!inst.book.asks.iter().any(|level| level.px == 105.0));
    }

    #[test]
    fn one_sided_finer_update_preserves_existing_scope() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(100.0, 10.0), lvl(95.0, 20.0)],
                asks: vec![lvl(105.0, 10.0), lvl(110.0, 20.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(99.0, 1.0)],
                asks: Vec::new(),
            },
            Some(1.0),
        );

        assert_eq!(
            inst.book
                .bids
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![100.0, 95.0]
        );
        assert_eq!(
            inst.book
                .asks
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![105.0, 110.0]
        );
    }

    #[test]
    fn empty_finer_update_preserves_existing_book() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(100.0, 10.0), lvl(95.0, 20.0)],
                asks: vec![lvl(105.0, 10.0), lvl(110.0, 20.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: Vec::new(),
                asks: Vec::new(),
            },
            Some(1.0),
        );

        assert_eq!(
            inst.book
                .bids
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![100.0, 95.0]
        );
        assert_eq!(
            inst.book
                .asks
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![105.0, 110.0]
        );
    }

    #[test]
    fn incomplete_finer_update_does_not_block_later_complete_update() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(100.0, 10.0), lvl(95.0, 20.0)],
                asks: vec![lvl(105.0, 10.0), lvl(110.0, 20.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(99.0, 1.0)],
                asks: Vec::new(),
            },
            Some(1.0),
        );
        assert_eq!(
            inst.book
                .asks
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![105.0, 110.0]
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(98.0, 2.0)],
                asks: vec![lvl(101.0, 3.0)],
            },
            Some(1.0),
        );

        assert_eq!(
            inst.book
                .bids
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![98.0, 95.0]
        );
        assert_eq!(
            inst.book
                .asks
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![101.0, 105.0, 110.0]
        );
        assert_eq!(inst.book.asks[0].sz, 3.0);
    }

    #[test]
    fn same_precision_empty_side_replaces_snapshot() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(100.0, 10.0), lvl(95.0, 20.0)],
                asks: vec![lvl(105.0, 10.0), lvl(110.0, 20.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(99.0, 1.0)],
                asks: Vec::new(),
            },
            Some(5.0),
        );

        assert_eq!(
            inst.book
                .bids
                .iter()
                .map(|level| level.px)
                .collect::<Vec<_>>(),
            vec![99.0]
        );
        assert!(inst.book.asks.is_empty());
    }

    #[test]
    fn same_precision_update_replaces_snapshot() {
        let mut inst = OrderBookInstance::new(0u64, OrderBookSymbolMode::Active, 5.0);
        inst.set_book_with_source(
            OrderBook {
                bids: vec![lvl(100.0, 10.0), lvl(95.0, 20.0)],
                asks: vec![lvl(105.0, 10.0), lvl(110.0, 20.0)],
            },
            Some(5.0),
        );

        inst.apply_book_update_preserving_scope(
            OrderBook {
                bids: vec![lvl(100.0, 1.0)],
                asks: vec![lvl(105.0, 1.0)],
            },
            Some(5.0),
        );

        assert_eq!(inst.book.bids.len(), 1);
        assert_eq!(inst.book.asks.len(), 1);
        assert_eq!(inst.book.bids[0].sz, 1.0);
        assert_eq!(inst.book.asks[0].sz, 1.0);
    }
}
