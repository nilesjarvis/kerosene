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
use std::collections::VecDeque;

// Layout constants for chart regions.
pub const PRICE_AXIS_WIDTH: f32 = 70.0;
pub const TIME_AXIS_HEIGHT: f32 = 24.0;
pub const DEFAULT_FUNDING_PANEL_HEIGHT: f32 = 56.0;
pub const MIN_FUNDING_PANEL_HEIGHT: f32 = 44.0;
pub const MAX_FUNDING_PANEL_HEIGHT: f32 = 220.0;
pub const DEFAULT_SESSION_PANEL_HEIGHT: f32 = 56.0;
pub const MIN_SESSION_PANEL_HEIGHT: f32 = 36.0;
pub const MAX_SESSION_PANEL_HEIGHT: f32 = 120.0;
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
pub(super) const HEATMAP_MAX_RECTS_WITH_FISHEYE: usize = 5_000;
pub(super) const HEATMAP_MAX_RECTS_WHILE_PANNING: usize = 4_000;

/// Heatmap tessellation budget for the current draw. Pan drags clear the
/// candle cache on every cursor event, so each drag frame re-tessellates the
/// full layer; capping the heatmap lower while a pan is active keeps those
/// frames cheap, and the gesture-end redraw restores full fidelity.
pub(super) fn heatmap_rect_budget(fisheye_distorts: bool, view_panning: bool) -> usize {
    let mut budget = HEATMAP_MAX_RECTS;
    if fisheye_distorts {
        budget = budget.min(HEATMAP_MAX_RECTS_WITH_FISHEYE);
    }
    if view_panning {
        budget = budget.min(HEATMAP_MAX_RECTS_WHILE_PANNING);
    }
    budget
}

pub struct CandlestickChart {
    pub id: u64,
    pub(in crate::chart) surface_id: ChartSurfaceId,
    pub(crate) symbol_key: String,
    pub(crate) symbol_label: String,
    /// Whether candle volume is denominated in whole contracts (outcome markets).
    pub(crate) whole_unit_volume: bool,
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
    /// SEC earnings-release dates rendered as chart time markers.
    pub earnings_markers: Vec<EarningsMarker>,
    /// Whether recent user fills should be rendered as trade dots.
    pub show_trade_markers: bool,
    /// Whether chart plot backgrounds use a dotted pattern instead of grid lines.
    pub(crate) dotted_background: bool,
    /// Opacity applied to dotted chart plot backgrounds.
    pub(crate) dotted_background_opacity: f32,
    /// Which candle bodies render hollow instead of filled.
    pub(crate) hollow_candle_mode: crate::config::ChartHollowCandleMode,
    /// Whether the main price series renders as candlesticks or a line + area fill.
    pub(crate) series_style: crate::config::ChartSeriesStyle,
    /// Whether chart plot geometry is rendered through a fisheye lens projection.
    pub(crate) fisheye_enabled: bool,
    /// Strength of the fisheye lens projection.
    pub(crate) fisheye_strength: f32,
    /// Whether chart geometry renders subtle red/cyan lens channel separation.
    pub(crate) chromatic_aberration_enabled: bool,
    /// Strength of the chromatic aberration channel separation.
    pub(crate) chromatic_aberration_strength: f32,
    /// Whether chart geometry renders subtle radial edge blur.
    pub(crate) edge_blur_enabled: bool,
    /// Strength of the radial edge blur effect.
    pub(crate) edge_blur_strength: f32,
    /// Reticle style used for the chart crosshair.
    pub(crate) crosshair_style: crate::config::ChartCrosshairStyle,
    /// Whether the chart crosshair draws full-span guide lines.
    pub(crate) crosshair_guides_enabled: bool,
    /// Multiplier applied to local crosshair reticle geometry.
    pub(crate) crosshair_scale: f32,
    /// HUD readout rows displayed around the central order type and size.
    pub(crate) hud_readout: crate::config::ChartHudReadoutConfig,
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
    /// Desired session indicator sub-panel height in pixels.
    pub session_panel_height: f32,
    /// Fresh executable mid/reference price used by HUD market-mode targeting.
    pub(crate) market_reference_price: Option<f64>,
    /// Fresh bid/ask spread from the chart asset context.
    pub(crate) current_spread: Option<f64>,
    /// Recent bid/ask spread samples used to scale the racing HUD spread gauge.
    pub(crate) spread_history: VecDeque<(u64, f64)>,
    /// Account-derived notional cap used to render HUD size-as-percent gauges.
    pub(crate) hud_max_notional: Option<f64>,
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
    /// Optional theme override for line-series visuals.
    pub chart_line_color: Option<Color>,
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
    /// Short visual pulse shown immediately after submitting a HUD chart order.
    pub(crate) hud_order_animation: Option<HudOrderAnimation>,
    /// Background loading pulses for market orders that are pending exchange response.
    pub(in crate::chart) pending_market_order_loading: Vec<MarketOrderLoadingOverlay>,
    /// True when HUD chart trading clicks are allowed to submit orders.
    pub(crate) hud_armed: bool,
    /// Looping 0..1 phase driving armed HUD pulse effects between animation ticks.
    pub(crate) hud_pulse_phase: f32,
    /// Last time the armed HUD chart was used or hovered.
    pub(crate) hud_last_activity_ms: Option<u64>,
    /// Whether the cursor is currently hovering over the HUD chart plot.
    pub(crate) hud_hovering: bool,
    /// True after the once-per-arm-session idle auto-disarm warning pip played.
    pub(crate) hud_idle_warning_sounded: bool,
    /// Recent HUD shots shown in the top-right battle feed (label, side, time).
    pub(crate) hud_feed: Vec<HudFeedEntry>,
    /// Transient weapon-selector popup shown after a mode/side keypress.
    pub(crate) hud_weapon_selector: Option<HudWeaponSelector>,
    /// Whether position entry and liquidation labels should be redacted while rendering.
    pub(crate) obscure_position_prices: bool,
    /// Whether active position/liquidation and order overlays should be hidden while rendering.
    pub(crate) hide_positions_and_orders: bool,
    /// OID of the chart order cancel button currently hovered by the cursor.
    pub(crate) hover_order_cancel_oid: Option<u64>,
    /// Smoothed hover progress for the chart order cancel button.
    pub(crate) order_cancel_hover_progress: f32,
    /// SEC earnings marker timestamp currently hovered by the cursor.
    pub(crate) hover_earnings_marker_time_ms: Option<u64>,
    /// Smoothed hover progress for the SEC earnings marker dot.
    pub(crate) earnings_marker_hover_progress: f32,
    /// Display-only conversion context for the chart header symbol price.
    pub(crate) display_denomination: DisplayDenominationContext,
}

impl CandlestickChart {
    pub(crate) fn price_axis_width(&self) -> f32 {
        PRICE_AXIS_WIDTH
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
    pub pending_state: Option<OrderOverlayPendingState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderOverlayPendingState {
    Placing,
    Cancelling,
    Modifying,
}

/// One row of the HUD battle feed: a recently fired chart order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HudFeedEntry {
    pub(crate) label: String,
    pub(crate) is_buy: bool,
    pub(crate) added_at_ms: u64,
}

/// Which loadout list the transient HUD weapon selector is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudSelectorKind {
    /// Order type swap: LIMIT / MARKET (the L/M "weapon switch").
    Mode,
    /// Market side: LONG / SHORT (the Y/X "fire selector").
    Side,
}

impl HudSelectorKind {
    pub(crate) fn for_control(control: crate::sound::HudUiSound) -> Option<Self> {
        use crate::sound::HudUiSound;
        match control {
            HudUiSound::ModeLimit | HudUiSound::ModeMarket => Some(Self::Mode),
            HudUiSound::SideLong | HudUiSound::SideShort => Some(Self::Side),
            _ => None,
        }
    }
}

/// Transient weapon-selector popup above the HUD station, Battlefield style:
/// pops open on a selector keypress, highlights the equipped slot, then
/// fades back to the persistent equipped-weapon readout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HudWeaponSelector {
    pub(crate) kind: HudSelectorKind,
    /// Normalized lifetime 0..1: pop-in, hold, fade-out; expires at 1.
    pub(crate) age: f32,
    /// 0..1 highlight pop of the just-selected slot.
    pub(crate) pop: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HudOrderAnimation {
    pub(crate) price: f64,
    pub(crate) origin_x: f32,
    pub(crate) click_y: f32,
    pub(crate) chart_w: f32,
    pub(crate) chart_h: f32,
    pub(crate) is_buy: bool,
    pub(crate) show_line: bool,
    pub(crate) progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct MarketOrderLoadingOverlay {
    pub(in crate::chart) pending_id: u64,
    pub(in crate::chart) is_buy: bool,
    pub(in crate::chart) progress: f32,
}

/// Lightweight user fill info passed to the chart for marker rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeMarker {
    pub time_ms: u64,
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
}

/// Lightweight SEC earnings event info passed to the chart for marker rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarningsMarker {
    pub time_ms: u64,
    pub filing_date: String,
    pub accession_number: String,
    pub quarter_label: Option<String>,
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
