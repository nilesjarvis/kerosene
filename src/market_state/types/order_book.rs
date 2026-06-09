use crate::account::AssetContext;
use crate::api::OrderBook;
use crate::helpers::{positive_finite_value, tick_sizes_match};

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Instant;

mod aggregation;
mod cache;
mod price_history;
mod scope;

pub use aggregation::AggregatedDepth;
pub(super) use aggregation::aggregate_with_cumulative;
use scope::merge_books_preserving_scope;

// ---------------------------------------------------------------------------
// Order Book State
// ---------------------------------------------------------------------------

pub type OrderBookId = u64;
pub(crate) const DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT: f32 = 60.0;
pub(crate) const MIN_ORDER_BOOK_SPREAD_CHART_HEIGHT: f32 = 30.0;
pub(crate) const MAX_ORDER_BOOK_SPREAD_CHART_HEIGHT: f32 = 1000.0;

pub(crate) fn clamp_order_book_spread_chart_height(height: f32) -> f32 {
    if height.is_finite() {
        height.clamp(
            MIN_ORDER_BOOK_SPREAD_CHART_HEIGHT,
            MAX_ORDER_BOOK_SPREAD_CHART_HEIGHT,
        )
    } else {
        DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT
    }
}

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
    pub reverse_side: bool,
    pub show_spread_chart: bool,
    pub spread_history: VecDeque<(Instant, f64)>,
    pub spread_chart_height: f32,
    pub(super) mid_price_history: VecDeque<(Instant, f64)>,
    book_source_tick_size: Option<f64>,
    book_source_mid: Option<f64>,
    pending_book_sigfigs: Option<(Option<u8>, Option<u8>)>,
    pub(super) book_revision: u64,
    aggregated: RefCell<AggregatedDepth>,
    dom_ladder: RefCell<super::super::dom_ladder::DomLadderCache>,
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
            reverse_side: false,
            show_spread_chart: false,
            spread_history: VecDeque::new(),
            spread_chart_height: DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT,
            mid_price_history: VecDeque::new(),
            book_source_tick_size: None,
            book_source_mid: None,
            pending_book_sigfigs: None,
            book_revision: 0,
            aggregated: RefCell::new(AggregatedDepth::default()),
            dom_ladder: RefCell::new(super::super::dom_ladder::DomLadderCache::default()),
        }
    }

    /// Replace the in-memory book snapshot. Bumps the revision counter so any
    /// cached aggregation is invalidated on the next read.
    pub fn set_book(&mut self, book: OrderBook) {
        self.set_book_with_source(book, None);
    }

    pub fn set_book_with_source(&mut self, book: OrderBook, source_tick_size: Option<f64>) {
        let source_mid = positive_finite_value(book.mid_price());
        self.book = book;
        self.book_source_tick_size = source_tick_size;
        self.book_source_mid = source_mid;
        self.book_revision = self.book_revision.wrapping_add(1);
    }

    pub fn book_source_tick_size(&self) -> Option<f64> {
        self.book_source_tick_size
    }

    pub fn book_source_mid(&self) -> Option<f64> {
        self.book_source_mid
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
            self.book = merge_books_preserving_scope(&self.book, &incoming);
            self.book_revision = self.book_revision.wrapping_add(1);
        } else {
            self.set_book_with_source(incoming, incoming_source_tick_size);
        }
    }

    pub fn set_tick_size(&mut self, tick: f64) {
        self.tick_size = tick;
    }

    pub(crate) fn set_spread_chart_height(&mut self, height: f32) {
        self.spread_chart_height = clamp_order_book_spread_chart_height(height);
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
}
