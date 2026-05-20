// ---------------------------------------------------------------------------
// Chart Model
// ---------------------------------------------------------------------------

use crate::annotations::{Annotation, DrawingTool};
use crate::api::Candle;
use crate::chart_state::ChartSurfaceId;
use crate::denomination::DisplayDenominationContext;
use crate::hydromancer_api::FundingRatePoint;
use crate::hyperdash_api::{HeatmapRect, LiquidationBucket};
use crate::timeframe::Timeframe;
use iced::Color;
use iced::widget::canvas;

// Layout constants for chart regions.
pub const PRICE_AXIS_WIDTH: f32 = 70.0;
pub const DISPLAY_DENOMINATED_PRICE_AXIS_WIDTH: f32 = 180.0;
pub const TIME_AXIS_HEIGHT: f32 = 24.0;
pub const DEFAULT_FUNDING_PANEL_HEIGHT: f32 = 56.0;
pub const MIN_FUNDING_PANEL_HEIGHT: f32 = 44.0;
pub const MAX_FUNDING_PANEL_HEIGHT: f32 = 220.0;
pub const MIN_MAIN_CHART_HEIGHT: f32 = 96.0;
pub const FUNDING_PANEL_RESIZE_HIT_PX: f32 = 5.0;
pub(in crate::chart) const FUNDING_MODE_BUTTON_X: f32 = 6.0;
pub(in crate::chart) const FUNDING_MODE_BUTTON_Y_OFFSET: f32 = 6.0;
pub(in crate::chart) const FUNDING_MODE_BUTTON_WIDTH: f32 = 38.0;
pub(in crate::chart) const FUNDING_MODE_BUTTON_HEIGHT: f32 = 15.0;
pub(in crate::chart) const FUNDING_PLOT_TOP_PADDING: f32 = 4.0;
pub(in crate::chart) const FUNDING_PLOT_BOTTOM_PADDING: f32 = 8.0;
pub(in crate::chart) const FUNDING_RATE_ANNUALIZATION_FACTOR: f64 = 24.0 * 365.0;
pub const VOLUME_REGION_RATIO: f32 = 0.18; // bottom 18% of chart area for volume bars
pub const PRICE_PADDING_PCT: f64 = 0.04; // 4% padding above/below price range

// Zoom / pan limits.
pub const MIN_CANDLE_WIDTH: f32 = 2.0;
pub const MAX_CANDLE_WIDTH: f32 = 60.0;
pub const DEFAULT_CANDLE_WIDTH: f32 = 10.0;
pub const CANDLE_GAP_RATIO: f32 = 0.2; // gap between candles as fraction of candle_width
pub const ZOOM_SPEED: f32 = 1.12; // multiplicative zoom per scroll tick
pub const PAN_SPEED: f32 = 1.0; // pixels of scroll -> candles of pan

pub(super) const HEATMAP_MAX_RECTS: usize = 20_000;

pub struct CandlestickChart {
    pub id: u64,
    pub(in crate::chart) surface_id: ChartSurfaceId,
    pub(in crate::chart) timeframe: Timeframe,
    pub(in crate::chart) clock_now_ms: u64,
    pub candles: Vec<Candle>,
    pub status: ChartStatus,
    pub candle_cache: canvas::Cache,
    pub(super) reset_epoch: u64,
    /// Active position on the currently viewed symbol (if any).
    pub active_position: Option<PositionOverlay>,
    /// Open limit orders on the currently viewed symbol.
    pub active_orders: Vec<OrderOverlay>,
    /// Recent user fills on the currently viewed symbol.
    pub trade_markers: Vec<TradeMarker>,
    /// Whether recent user fills should be rendered as trade dots.
    pub show_trade_markers: bool,
    /// User-drawn annotations (levels, trend lines).
    pub annotations: Vec<Annotation>,
    /// Currently active drawing tool (None = normal pan/zoom mode).
    pub active_tool: Option<DrawingTool>,
    /// Aggregated liquidation heatmap buckets (computed from LiquidationLevel data).
    pub liquidation_buckets: Vec<LiquidationBucket>,
    /// Historical liquidation heatmap cells for time-based rendering.
    pub heatmap_rects: Vec<HeatmapRect>,
    /// Max absolute USD value for heatmap color normalization.
    pub heatmap_max_usd: f64,
    /// Funding-rate history rendered in the optional funding sub-panel.
    pub funding_rates: Vec<FundingRatePoint>,
    /// Optional funding sub-panel status text and error flag.
    pub funding_status: Option<(String, bool)>,
    /// Desired funding sub-panel height in pixels.
    pub funding_panel_height: f32,
    /// Render funding as hourly rate or annualized rate.
    pub funding_annualized: bool,
    // Macro MAs
    pub macro_indicators: crate::config::MacroIndicatorsConfig,
    pub daily_candles: Vec<crate::api::Candle>,
    pub weekly_candles: Vec<crate::api::Candle>,
    pub monthly_candles: Vec<crate::api::Candle>,
    /// Inverted visual mode (price axis flipped vertically).
    pub inverted: bool,
    /// Optional theme override for bullish candle visuals.
    pub chart_bull_color: Option<Color>,
    /// Optional theme override for bearish candle visuals.
    pub chart_bear_color: Option<Color>,
    /// Whether the quick-order card is open over this chart. When true, left-clicks
    /// inside the chart canvas close the card while right-clicks still publish a
    /// replacement quick-order at the clicked price.
    pub(crate) quick_order_open: bool,
    /// Temporary limit price preview shown while the chart quick-order card is open.
    pub(crate) quick_order_limit_price: Option<f64>,
    /// Pixel phase for animating the temporary quick-order limit preview line.
    pub(crate) quick_order_line_phase: f32,
    /// Pixel phase for animating active order lines while moving them.
    pub(crate) order_line_phase: f32,
    /// Whether position entry and liquidation labels should be redacted while rendering.
    pub(crate) obscure_position_prices: bool,
    /// Whether active position/liquidation and order overlays should be hidden while rendering.
    pub(crate) hide_positions_and_orders: bool,
    /// Display-only conversion context for USD-denominated chart labels.
    pub(crate) display_denomination: DisplayDenominationContext,
}

impl CandlestickChart {
    pub(crate) fn price_axis_width(&self) -> f32 {
        if self.display_denomination.includes_usd_chart_reference() {
            DISPLAY_DENOMINATED_PRICE_AXIS_WIDTH
        } else {
            PRICE_AXIS_WIDTH
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartViewport {
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub price_lo: f64,
    pub price_hi: f64,
    pub chart_width: f32,
    pub candle_width: f32,
    pub scroll_offset: f32,
    pub y_auto: bool,
    pub y_scale: f64,
    pub y_offset: f64,
    pub funding_y_scale: f64,
    pub funding_y_offset: f64,
}

/// Lightweight position info passed to the chart for overlay rendering.
#[derive(Debug, Clone)]
pub struct PositionOverlay {
    pub entry_px: f64,
    pub szi: f64, // positive = long, negative = short
    pub liquidation_px: Option<f64>,
}

/// Lightweight open order info passed to the chart for overlay rendering.
#[derive(Debug, Clone)]
pub struct OrderOverlay {
    pub coin: String,
    pub limit_px: f64,
    pub sz: f64,
    pub is_buy: bool,
    pub oid: u64,
    pub is_moving: bool,
}

/// Lightweight user fill info passed to the chart for marker rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeMarker {
    pub time_ms: u64,
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
}

/// Status of chart data loading.
#[derive(Debug, Clone)]
pub enum ChartStatus {
    Loading,
    Loaded,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VisibleCandleRange {
    pub(super) first: usize,
    pub(super) last: usize,
    pub(super) right_idx: isize,
}
