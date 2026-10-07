use crate::account::AssetContext;
use crate::api::OrderBook;
use crate::config;
use crate::helpers::{
    book_tick_options, default_tick_for_price, nearest_tick_option, positive_finite_value,
};
use crate::market_state::MARKET_ASSET_CONTEXT_MAX_AGE_MS;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

mod aggregation;
mod cache;
mod price_history;
mod requests;

#[cfg(test)]
mod tests;

pub use aggregation::AggregatedDepth;
pub(super) use aggregation::aggregate_with_cumulative;
use requests::PendingOrderBookRequest;

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
    DepthChart,
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
    pub(crate) asset_ctx_updated_at: Option<Instant>,
    pub scroll_id: iced::widget::Id,
    pub tick_size: f64,
    pub settings_open: bool,
    pub search_query: String,
    pub display_mode: OrderBookDisplayMode,
    pub book_loading: bool,
    pub book_error: Option<String>,
    /// Set once a load failure has been toasted, so a failing background
    /// refresh loop produces one toast per streak instead of one per attempt.
    /// Cleared by a successful snapshot or a symbol/mode change.
    pub book_failure_toasted: bool,
    pub center_on_mid: bool,
    pub reverse_side: bool,
    pub show_spread_chart: bool,
    pub spread_history: VecDeque<(Instant, f64)>,
    pub spread_chart_height: f32,
    pub(super) mid_price_history: VecDeque<(Instant, f64)>,
    book_source_tick_size: Option<f64>,
    book_source_mid: Option<f64>,
    displayed_tick_size: f64,
    /// Basis for native API tick options. Stable within a price decade; when
    /// the decade changes, preserve the selected precision's relative scale.
    tick_options_basis: Option<f64>,
    next_book_request_id: u64,
    pending_book_request: Option<PendingOrderBookRequest>,
    pub(super) book_revision: u64,
    aggregated: RefCell<AggregatedDepth>,
    dom_ladder: RefCell<super::super::dom_ladder::DomLadderCache>,
}

impl OrderBookInstance {
    /// Restore settings using the caller's resolved symbol mode and tick size.
    pub(crate) fn from_config(
        config: &config::OrderBookConfig,
        mode: OrderBookSymbolMode,
        tick_size: f64,
    ) -> Self {
        let mut instance = Self::new(config.id, mode, tick_size);
        instance.display_mode = match config.display_mode {
            config::OrderBookDisplayModeConfig::DepthList => OrderBookDisplayMode::DepthList,
            config::OrderBookDisplayModeConfig::DomLadder => OrderBookDisplayMode::DomLadder,
            config::OrderBookDisplayModeConfig::DepthChart => OrderBookDisplayMode::DepthChart,
        };
        instance.center_on_mid = config.center_on_mid;
        instance.reverse_side = config.reverse_side;
        instance.show_spread_chart = config.show_spread_chart;
        instance.set_spread_chart_height(config.spread_chart_height);
        instance.book_loading = true;
        instance
    }

    pub fn new(id: OrderBookId, mode: OrderBookSymbolMode, tick_size: f64) -> Self {
        Self {
            id,
            mode,
            book: OrderBook::empty(),
            asset_ctx: None,
            asset_ctx_updated_at: None,
            scroll_id: iced::widget::Id::unique(),
            tick_size,
            settings_open: false,
            search_query: String::new(),
            display_mode: OrderBookDisplayMode::DepthList,
            book_loading: false,
            book_error: None,
            book_failure_toasted: false,
            // Pinned-spread view by default: it is the layout that stays
            // readable while the market moves. Scrollable mode remains one
            // Center-toggle away.
            center_on_mid: true,
            reverse_side: false,
            show_spread_chart: false,
            spread_history: VecDeque::new(),
            spread_chart_height: DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT,
            mid_price_history: VecDeque::new(),
            book_source_tick_size: None,
            book_source_mid: None,
            displayed_tick_size: tick_size,
            tick_options_basis: None,
            next_book_request_id: 0,
            pending_book_request: None,
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
        self.displayed_tick_size = source_tick_size.unwrap_or(self.tick_size);
        if let Some(mid) = source_mid {
            self.update_tick_options_basis(mid);
        }
        self.book_revision = self.book_revision.wrapping_add(1);
    }

    /// Mid price the tick-size options should be derived from: the sticky
    /// basis when one is known, otherwise the live book mid.
    pub fn tick_options_mid(&self) -> f64 {
        self.tick_options_basis
            .unwrap_or_else(|| self.book.mid_price())
    }

    pub fn reset_tick_options_basis(&mut self) {
        self.tick_options_basis = None;
    }

    /// Drop the asset context (e.g. on stream lag or staleness). Book-derived
    /// price history (spread, mid-price) is left intact: it is refreshed by
    /// the live L2 book and trimmed by its own time window, so an
    /// asset-context hiccup must not blank the spread chart or the
    /// short-term price move.
    pub fn clear_asset_context(&mut self) {
        self.asset_ctx = None;
        self.asset_ctx_updated_at = None;
    }

    /// Drop the asset context and all book-derived price history. Used when the
    /// symbol changes or the book is reset, so samples from the previous symbol
    /// never bleed into the new one.
    pub fn clear_asset_context_and_price_history(&mut self) {
        self.clear_asset_context();
        self.clear_spread_history();
        self.clear_mid_price_history();
    }

    pub fn expire_asset_context_if_stale(&mut self, now: Instant) -> bool {
        let Some(updated_at) = self.asset_ctx_updated_at else {
            return false;
        };
        if self.asset_ctx.is_none() {
            self.asset_ctx_updated_at = None;
            return false;
        }
        if now
            .checked_duration_since(updated_at)
            .is_some_and(|age| age > Duration::from_millis(MARKET_ASSET_CONTEXT_MAX_AGE_MS))
        {
            self.clear_asset_context();
            return true;
        }
        false
    }

    fn update_tick_options_basis(&mut self, mid: f64) {
        if let Some(basis) = self.tick_options_basis {
            let old_base = default_tick_for_price(basis);
            let new_base = default_tick_for_price(mid);
            if crate::helpers::tick_sizes_match(old_base, new_base) {
                return;
            }
            self.tick_size *= new_base / old_base;
        }
        self.tick_options_basis = Some(mid);
    }

    pub fn book_source_tick_size(&self) -> Option<f64> {
        self.book_source_tick_size
    }

    pub fn book_source_mid(&self) -> Option<f64> {
        self.book_source_mid
    }

    /// Hold the displayed denomination until its replacement snapshot arrives,
    /// in either direction. Re-bucketing a fine snapshot immediately would
    /// collapse the visible depth while a wider book is still in flight.
    pub fn displayed_tick_size(&self) -> f64 {
        self.displayed_tick_size
    }

    pub(crate) fn normalize_tick_size(&mut self) {
        if positive_finite_value(self.tick_options_mid()).is_some() {
            self.tick_size =
                nearest_tick_option(&book_tick_options(self.tick_options_mid()), self.tick_size);
        }
    }

    pub fn set_tick_size(&mut self, tick: f64) {
        self.tick_size = tick;
    }

    pub(crate) fn set_spread_chart_height(&mut self, height: f32) {
        self.spread_chart_height = clamp_order_book_spread_chart_height(height);
    }
}
