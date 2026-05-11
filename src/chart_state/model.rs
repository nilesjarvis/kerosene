use crate::account::AssetContext;
use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart::{CandlestickChart, ChartViewport};
use crate::config;
use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap, LiquidationLevel};
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

pub(crate) type ChartId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandleFetchRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) symbol: String,
    pub(crate) timeframe: Timeframe,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) attempt: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FundingFetchRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) symbol: String,
    pub(crate) coin: String,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) mode: FundingFetchMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FundingFetchMode {
    Snapshot,
    Incremental,
}

/// Per-chart instance state. Each chart pane has its own symbol, timeframe,
/// candle data, and WebSocket subscriptions.
pub(crate) struct ChartInstance {
    pub(crate) id: ChartId,
    pub(crate) symbol: String,
    pub(crate) symbol_display: String,
    pub(crate) interval: Timeframe,
    pub(crate) chart: CandlestickChart,
    pub(crate) asset_ctx: Option<AssetContext>,
    /// Whether the inline symbol editor is open.
    pub(crate) editor_open: bool,
    /// Search query text for the symbol editor.
    pub(crate) editor_search_query: String,
    /// Whether the top editor result is armed for keyboard selection.
    pub(crate) editor_keyboard_selected: bool,
    /// Right-click quick order form (if open).
    pub(crate) quick_order: Option<QuickOrderForm>,
    /// Remember the last quick order type (true=limit, false=market).
    pub(crate) last_quick_order_is_limit: bool,
    /// User-drawn annotations (persisted).
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) next_annotation_id: AnnotationId,
    /// Whether the liquidation level overlay is enabled for this chart.
    pub(crate) show_liquidations: bool,
    /// Cached liquidation level data from HyperDash API.
    pub(crate) liquidation_data: Option<LiquidationLevel>,
    /// Whether current liquidation levels are currently being fetched.
    pub(crate) liquidation_fetching: bool,
    /// Last LIQ status shown near the chart controls.
    pub(crate) liquidation_status: Option<(String, bool)>,
    /// Shared request key for the latest in-flight LIQ request.
    pub(crate) liquidation_pending_key: Option<String>,
    /// Whether the historical liquidation heatmap is enabled for this chart.
    pub(crate) show_heatmap: bool,
    /// Cached heatmap data from HyperDash API.
    pub(crate) heatmap_data: Option<LiquidationHeatmap>,
    /// Parameters of the last heatmap fetch (for detecting range changes).
    pub(crate) heatmap_last_fetch: Option<HeatmapFetchParams>,
    /// Last visible chart viewport reported by the canvas.
    pub(crate) heatmap_viewport: Option<ChartViewport>,
    /// Persistent heatmap status shown near the chart controls.
    pub(crate) heatmap_status: Option<(String, bool)>,
    /// Whether a heatmap fetch is currently in flight.
    pub(crate) heatmap_fetching: bool,
    /// Latest in-flight historical candle request for stale-response guards.
    pub(crate) candle_fetch_request: Option<CandleFetchRequest>,
    /// Non-blocking refresh error shown while previously loaded candles remain visible.
    pub(crate) candle_fetch_error: Option<String>,
    /// Latest in-flight funding history request for stale-response guards.
    pub(crate) funding_fetch_request: Option<FundingFetchRequest>,
    /// Last time this chart attempted a funding history fetch.
    pub(crate) funding_last_attempt_ms: Option<u64>,
    /// Active macro indicators configuration.
    pub(crate) macro_indicators: config::MacroIndicatorsConfig,
    /// Toggle state for the macro indicators dropdown menu.
    pub(crate) macro_menu_open: bool,
    /// Whether the header open-interest metric is shown as USD notional for this chart.
    pub(crate) open_interest_as_notional: bool,
}

impl ChartInstance {
    pub(crate) fn new(id: ChartId, symbol: String, interval: Timeframe) -> Self {
        let display = symbol.split(':').nth(1).unwrap_or(&symbol).to_string();
        Self {
            id,
            symbol,
            symbol_display: display,
            interval,
            chart: CandlestickChart::new(id),
            asset_ctx: None,
            editor_open: false,
            editor_search_query: String::new(),
            editor_keyboard_selected: false,
            quick_order: None,
            last_quick_order_is_limit: true,
            annotations: Vec::new(),
            next_annotation_id: 0,
            show_liquidations: false,
            liquidation_data: None,
            liquidation_fetching: false,
            liquidation_status: None,
            liquidation_pending_key: None,
            show_heatmap: false,
            heatmap_data: None,
            heatmap_last_fetch: None,
            heatmap_viewport: None,
            heatmap_status: None,
            heatmap_fetching: false,
            candle_fetch_request: None,
            candle_fetch_error: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
        }
    }

    /// Create a new chart with the editor open and no symbol selected.
    pub(crate) fn new_empty(id: ChartId) -> Self {
        Self {
            id,
            symbol: String::new(),
            symbol_display: String::new(),
            interval: Timeframe::H1,
            chart: CandlestickChart::new(id),
            asset_ctx: None,
            editor_open: true,
            editor_search_query: String::new(),
            editor_keyboard_selected: false,
            quick_order: None,
            last_quick_order_is_limit: true,
            annotations: Vec::new(),
            next_annotation_id: 0,
            show_liquidations: false,
            liquidation_data: None,
            liquidation_fetching: false,
            liquidation_status: None,
            liquidation_pending_key: None,
            show_heatmap: false,
            heatmap_data: None,
            heatmap_last_fetch: None,
            heatmap_viewport: None,
            heatmap_status: None,
            heatmap_fetching: false,
            candle_fetch_request: None,
            candle_fetch_error: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
        }
    }
}

impl TradingTerminal {
    /// Allocate the next chart ID.
    pub(crate) fn alloc_chart_id(&mut self) -> ChartId {
        let _theme = self.theme();
        let id = self.next_chart_id;
        self.next_chart_id += 1;
        id
    }
}
