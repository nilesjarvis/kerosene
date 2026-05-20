use crate::account::AssetContext;
use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart::{CandlestickChart, ChartViewport};
use crate::config;
use crate::hyperdash_api::{HeatmapFetchParams, LiquidationHeatmap, LiquidationLevel};
use crate::order_execution::QuickOrderForm;
use crate::timeframe::Timeframe;
use iced::{Point, Size, window};

pub(crate) type ChartId = u64;

pub(crate) const CHART_PRICE_FLASH_MS: u64 = 800;
const QUICK_ORDER_LIMIT_LINE_STRIDE: f32 = 12.0;
const QUICK_ORDER_LIMIT_LINE_PHASE_STEP: f32 = 1.2;
const ORDER_LINE_STRIDE: f32 = 12.0;
const ORDER_LINE_PHASE_STEP: f32 = 1.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ChartSurfaceId {
    Docked(ChartId),
    Detached(window::Id),
}

impl ChartSurfaceId {
    pub(crate) fn widget_suffix(self) -> String {
        match self {
            Self::Docked(chart_id) => format!("docked_{chart_id}"),
            Self::Detached(window_id) => format!("detached_{window_id:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DetachedChartWindowState {
    pub(crate) chart_id: ChartId,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
}

impl DetachedChartWindowState {
    pub(crate) fn new(chart_id: ChartId) -> Self {
        Self {
            chart_id,
            width: crate::config::default_detached_chart_window_width(),
            height: crate::config::default_detached_chart_window_height(),
            x: None,
            y: None,
        }
    }

    pub(crate) fn from_config(config: &crate::config::DetachedChartWindowConfig) -> Self {
        Self {
            chart_id: config.chart_id,
            width: normalize_detached_chart_window_dimension(
                config.width,
                crate::config::default_detached_chart_window_width(),
            ),
            height: normalize_detached_chart_window_dimension(
                config.height,
                crate::config::default_detached_chart_window_height(),
            ),
            x: config.x.filter(|value| value.is_finite()),
            y: config.y.filter(|value| value.is_finite()),
        }
    }

    pub(crate) fn to_config(&self) -> crate::config::DetachedChartWindowConfig {
        crate::config::DetachedChartWindowConfig {
            chart_id: self.chart_id,
            width: normalize_detached_chart_window_dimension(
                self.width,
                crate::config::default_detached_chart_window_width(),
            ),
            height: normalize_detached_chart_window_dimension(
                self.height,
                crate::config::default_detached_chart_window_height(),
            ),
            x: self.x.filter(|value| value.is_finite()),
            y: self.y.filter(|value| value.is_finite()),
        }
    }

    pub(crate) fn size(&self) -> Size {
        Size::new(
            normalize_detached_chart_window_dimension(
                self.width,
                crate::config::default_detached_chart_window_width(),
            ),
            normalize_detached_chart_window_dimension(
                self.height,
                crate::config::default_detached_chart_window_height(),
            ),
        )
    }

    pub(crate) fn position(&self) -> window::Position {
        self.x
            .zip(self.y)
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .map(|(x, y)| window::Position::Specific(Point::new(x, y)))
            .unwrap_or(window::Position::Centered)
    }
}

fn normalize_detached_chart_window_dimension(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(320.0)
    } else {
        fallback
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriceFlashDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PriceFlash {
    pub(crate) started_at_ms: u64,
    pub(crate) direction: PriceFlashDirection,
    pub(crate) previous_close: f64,
}

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
        let mut chart = CandlestickChart::new(id);
        chart.set_timeframe(interval);
        Self {
            id,
            symbol,
            symbol_display: display,
            interval,
            chart,
            asset_ctx: None,
            editor_open: false,
            editor_search_query: String::new(),
            editor_keyboard_selected: false,
            quick_order: None,
            last_quick_order_is_limit: true,
            last_quick_order_symbol: String::new(),
            last_quick_order_quantity: String::new(),
            last_quick_order_quantity_is_usd: false,
            last_quick_order_percentage: 0.0,
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
            last_price_flash: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
        }
    }

    pub(crate) fn quick_order_reopen_values(
        &self,
        fallback_quantity_is_usd: bool,
    ) -> (String, bool, f32, bool) {
        if let Some(form) = &self.quick_order {
            return (
                form.quantity.clone(),
                form.quantity_is_usd,
                form.percentage,
                form.is_limit,
            );
        }

        if self.last_quick_order_symbol == self.symbol {
            return (
                self.last_quick_order_quantity.clone(),
                self.last_quick_order_quantity_is_usd,
                self.last_quick_order_percentage,
                self.last_quick_order_is_limit,
            );
        }

        (
            String::new(),
            fallback_quantity_is_usd,
            0.0,
            self.last_quick_order_is_limit,
        )
    }

    pub(crate) fn set_quick_order(&mut self, form: QuickOrderForm) {
        self.remember_quick_order_form(&form);
        self.chart.quick_order_limit_price = form.is_limit.then_some(form.price);
        self.chart.quick_order_line_phase = 0.0;
        self.quick_order = Some(form);
        self.chart.quick_order_open = true;
    }

    pub(crate) fn clear_quick_order(&mut self) {
        self.remember_current_quick_order();
        self.quick_order = None;
        self.chart.quick_order_open = false;
        self.chart.quick_order_limit_price = None;
        self.chart.quick_order_line_phase = 0.0;
    }

    pub(crate) fn take_quick_order(&mut self) -> Option<QuickOrderForm> {
        self.remember_current_quick_order();
        let form = self.quick_order.take();
        self.chart.quick_order_open = false;
        self.chart.quick_order_limit_price = None;
        self.chart.quick_order_line_phase = 0.0;
        form
    }

    fn remember_current_quick_order(&mut self) {
        let Some(form) = self.quick_order.as_ref() else {
            return;
        };
        let quantity = form.quantity.clone();
        let quantity_is_usd = form.quantity_is_usd;
        let percentage = form.percentage;
        let is_limit = form.is_limit;

        self.last_quick_order_symbol = self.symbol.clone();
        self.last_quick_order_quantity = quantity;
        self.last_quick_order_quantity_is_usd = quantity_is_usd;
        self.last_quick_order_percentage = percentage;
        self.last_quick_order_is_limit = is_limit;
    }

    fn remember_quick_order_form(&mut self, form: &QuickOrderForm) {
        self.last_quick_order_symbol = self.symbol.clone();
        self.last_quick_order_quantity = form.quantity.clone();
        self.last_quick_order_quantity_is_usd = form.quantity_is_usd;
        self.last_quick_order_percentage = form.percentage;
        self.last_quick_order_is_limit = form.is_limit;
    }

    pub(crate) fn advance_quick_order_limit_line(&mut self) {
        if self.chart.quick_order_limit_price.is_some() {
            self.chart.quick_order_line_phase = (self.chart.quick_order_line_phase
                + QUICK_ORDER_LIMIT_LINE_PHASE_STEP)
                .rem_euclid(QUICK_ORDER_LIMIT_LINE_STRIDE);
        }
    }

    pub(crate) fn advance_order_line_animation(&mut self) {
        if !self.chart.active_orders.is_empty() {
            self.chart.order_line_phase =
                (self.chart.order_line_phase + ORDER_LINE_PHASE_STEP).rem_euclid(ORDER_LINE_STRIDE);
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
            last_quick_order_symbol: String::new(),
            last_quick_order_quantity: String::new(),
            last_quick_order_quantity_is_usd: false,
            last_quick_order_percentage: 0.0,
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
            last_price_flash: None,
            funding_fetch_request: None,
            funding_last_attempt_ms: None,
            macro_indicators: config::MacroIndicatorsConfig::default(),
            macro_menu_open: false,
            open_interest_as_notional: false,
        }
    }

    pub(crate) fn track_last_price_update(
        &mut self,
        previous_close: Option<f64>,
        next_close: f64,
        now_ms: u64,
    ) {
        let Some(previous_close) = previous_close else {
            return;
        };
        if !previous_close.is_finite() || !next_close.is_finite() {
            return;
        }
        if (next_close - previous_close).abs() <= f64::EPSILON {
            return;
        }
        let direction = if next_close > previous_close {
            PriceFlashDirection::Up
        } else {
            PriceFlashDirection::Down
        };
        self.last_price_flash = Some(PriceFlash {
            started_at_ms: now_ms,
            direction,
            previous_close,
        });
    }

    pub(crate) fn last_price_flash_is_active(&self, now_ms: u64) -> bool {
        self.last_price_flash
            .is_some_and(|flash| now_ms.saturating_sub(flash.started_at_ms) < CHART_PRICE_FLASH_MS)
    }

    pub(crate) fn clear_expired_last_price_flash(&mut self, now_ms: u64) {
        if self.last_price_flash_is_active(now_ms) {
            return;
        }
        self.last_price_flash = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> ChartInstance {
        ChartInstance::new(1, "BTC".to_string(), Timeframe::H1)
    }

    #[test]
    fn chart_surface_widget_suffixes_distinguish_docked_charts() {
        assert_ne!(
            ChartSurfaceId::Docked(1).widget_suffix(),
            ChartSurfaceId::Docked(2).widget_suffix()
        );
    }

    #[test]
    fn last_price_flash_tracks_websocket_direction() {
        let mut instance = instance();

        instance.track_last_price_update(Some(100.0), 101.0, 42);
        assert_eq!(
            instance.last_price_flash,
            Some(PriceFlash {
                started_at_ms: 42,
                direction: PriceFlashDirection::Up,
                previous_close: 100.0,
            })
        );

        instance.track_last_price_update(Some(101.0), 99.0, 84);
        assert_eq!(
            instance.last_price_flash,
            Some(PriceFlash {
                started_at_ms: 84,
                direction: PriceFlashDirection::Down,
                previous_close: 101.0,
            })
        );
    }

    #[test]
    fn last_price_flash_ignores_missing_or_unchanged_prices() {
        let mut instance = instance();

        instance.track_last_price_update(None, 101.0, 42);
        assert!(instance.last_price_flash.is_none());

        instance.track_last_price_update(Some(101.0), 101.0, 42);
        assert!(instance.last_price_flash.is_none());
    }

    #[test]
    fn last_price_flash_expires_after_flash_window() {
        let mut instance = instance();

        instance.track_last_price_update(Some(100.0), 101.0, 1_000);
        assert!(instance.last_price_flash_is_active(1_000 + CHART_PRICE_FLASH_MS - 1));

        instance.clear_expired_last_price_flash(1_000 + CHART_PRICE_FLASH_MS);
        assert!(instance.last_price_flash.is_none());
    }

    #[test]
    fn quick_order_limit_preview_tracks_open_form_lifecycle() {
        let mut instance = instance();

        instance.set_quick_order(QuickOrderForm {
            price: 100.0,
            quantity: String::new(),
            quantity_is_usd: false,
            percentage: 0.0,
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });
        assert!(instance.chart.quick_order_open);
        assert_eq!(instance.chart.quick_order_limit_price, Some(100.0));

        instance.clear_quick_order();
        assert!(!instance.chart.quick_order_open);
        assert_eq!(instance.chart.quick_order_limit_price, None);
    }

    #[test]
    fn quick_order_reopen_values_preserve_cleared_form_for_same_symbol() {
        let mut instance = instance();

        instance.set_quick_order(QuickOrderForm {
            price: 100.0,
            quantity: "2.5".to_string(),
            quantity_is_usd: false,
            percentage: 25.0,
            is_limit: false,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });
        instance.clear_quick_order();

        assert_eq!(
            instance.quick_order_reopen_values(true),
            ("2.5".to_string(), false, 25.0, false)
        );
    }

    #[test]
    fn quick_order_reopen_values_drop_size_after_symbol_change() {
        let mut instance = instance();

        instance.set_quick_order(QuickOrderForm {
            price: 100.0,
            quantity: "2.5".to_string(),
            quantity_is_usd: false,
            percentage: 25.0,
            is_limit: false,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });
        instance.clear_quick_order();
        instance.symbol = "ETH".to_string();

        assert_eq!(
            instance.quick_order_reopen_values(true),
            (String::new(), true, 0.0, false)
        );
    }

    #[test]
    fn quick_order_market_form_does_not_show_limit_preview() {
        let mut instance = instance();

        instance.set_quick_order(QuickOrderForm {
            price: 100.0,
            quantity: String::new(),
            quantity_is_usd: false,
            percentage: 0.0,
            is_limit: false,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });

        assert!(instance.chart.quick_order_open);
        assert_eq!(instance.chart.quick_order_limit_price, None);
    }

    #[test]
    fn quick_order_limit_preview_phase_only_advances_while_visible() {
        let mut instance = instance();

        instance.advance_quick_order_limit_line();
        assert_eq!(instance.chart.quick_order_line_phase, 0.0);

        instance.set_quick_order(QuickOrderForm {
            price: 100.0,
            quantity: String::new(),
            quantity_is_usd: false,
            percentage: 0.0,
            is_limit: true,
            click_x: 10.0,
            click_y: 20.0,
            chart_w: 300.0,
            chart_h: 200.0,
        });
        instance.advance_quick_order_limit_line();
        assert!(instance.chart.quick_order_line_phase > 0.0);

        instance.clear_quick_order();
        assert_eq!(instance.chart.quick_order_line_phase, 0.0);
    }
}
