use crate::account::AssetContext;
use crate::annotations::{Annotation, AnnotationId};
use crate::api::SecEarningsEvent;
use crate::app_state::TradingTerminal;
use crate::chart::{CandlestickChart, ChartViewport};
use crate::config;
use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap, LiquidationLevel};
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;

pub(crate) type ChartId = u64;

mod instance;
mod order_lines;
mod price_flash;
mod quick_order;
mod surface;

pub(crate) use price_flash::{CHART_PRICE_FLASH_MS, PriceFlash, PriceFlashDirection};
pub(crate) use surface::{ChartSurfaceId, DetachedChartWindowState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandleFetchRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) symbol: String,
    pub(crate) timeframe: Timeframe,
    pub(crate) source: config::ChartBackfillSource,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) attempt: u8,
}

#[derive(Clone)]
pub(crate) struct ChartBackfillFetchContext {
    pub(crate) source: config::ChartBackfillSource,
    pub(crate) hydromancer_api_key: String,
}

impl ChartBackfillFetchContext {
    pub(crate) fn new(source: config::ChartBackfillSource, hydromancer_api_key: String) -> Self {
        Self {
            source,
            hydromancer_api_key,
        }
    }
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
    /// Whether the chart header is collapsed to a ticker-only strip.
    pub(crate) header_collapsed: bool,
    /// Search query text for the symbol editor.
    pub(crate) editor_search_query: String,
    /// Search result index currently highlighted for keyboard selection.
    pub(crate) editor_selected_index: Option<usize>,
    /// Right-click quick order form (if open).
    pub(crate) quick_order: Option<QuickOrderForm>,
    /// Remember the last quick order type (true=limit, false=market).
    pub(crate) last_quick_order_is_limit: bool,
    /// Symbol the cached quick-order size belongs to.
    pub(crate) last_quick_order_symbol: String,
    /// Last quick-order quantity text for reopening the form on the same symbol.
    pub(crate) last_quick_order_quantity: String,
    /// Last quick-order quantity denomination.
    pub(crate) last_quick_order_quantity_is_usd: bool,
    /// Last quick-order percentage slider value.
    pub(crate) last_quick_order_percentage: f32,
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
    /// Transient direction flash for price-derived header numbers after WS updates.
    pub(crate) last_price_flash: Option<PriceFlash>,
    /// Whether SEC earnings-release markers are enabled for this chart.
    pub(crate) show_earnings_markers: bool,
    /// Cached SEC earnings data applied to this chart.
    pub(crate) earnings_events: Option<Vec<SecEarningsEvent>>,
    /// Whether SEC earnings events are currently being fetched for this chart.
    pub(crate) earnings_fetching: bool,
    /// Persistent earnings status shown near the chart controls.
    pub(crate) earnings_status: Option<(String, bool)>,
    /// Ticker for the latest in-flight SEC earnings request.
    pub(crate) earnings_pending_ticker: Option<String>,
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
    /// Whether the HIP-4 volume metric is shown as USD notional for this chart.
    pub(crate) outcome_volume_as_notional: bool,
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

#[cfg(test)]
mod tests;
