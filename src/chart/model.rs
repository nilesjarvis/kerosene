// ---------------------------------------------------------------------------
// Chart Model
// ---------------------------------------------------------------------------

use crate::annotations::{Annotation, DrawingTool};
use crate::api::Candle;
use crate::hyperdash_api::{HeatmapRect, LiquidationBucket};
use iced::Color;
use iced::widget::canvas;

// Layout constants for chart regions.
pub const PRICE_AXIS_WIDTH: f32 = 70.0;
pub const TIME_AXIS_HEIGHT: f32 = 24.0;
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
    pub candles: Vec<Candle>,
    pub status: ChartStatus,
    pub candle_cache: canvas::Cache,
    pub(super) reset_epoch: u64,
    /// Active position on the currently viewed symbol (if any).
    pub active_position: Option<PositionOverlay>,
    /// Open limit orders on the currently viewed symbol.
    pub active_orders: Vec<OrderOverlay>,
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartViewport {
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub price_lo: f64,
    pub price_hi: f64,
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
