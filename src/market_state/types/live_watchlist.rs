use crate::config;

// ---------------------------------------------------------------------------
// Live Watchlist Types
// ---------------------------------------------------------------------------

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
    pub(crate) ema_distance: Option<f64>,
    pub(crate) ema_status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LiveWatchlistInstance {
    pub id: LiveWatchlistId,
    pub preset_id: Option<config::WatchlistPresetId>,
    pub symbols: Vec<String>,
    pub search_query: String,
    pub sort_column: config::LiveWatchlistSortColumn,
    pub sort_direction: config::SortDirection,
    pub visible_columns: Vec<config::LiveWatchlistColumn>,
    pub ema: config::LiveWatchlistEmaConfig,
    pub ema_period_input: String,
    pub(crate) row_cache: Vec<LiveWatchlistRowData>,
}
