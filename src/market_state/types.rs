use crate::account::AssetContext;
use crate::api::OrderBook;
use crate::config;

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
        }
    }
}
