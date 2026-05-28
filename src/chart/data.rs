mod candles;
mod funding;

use super::{CandlestickChart, ChartStatus, DEFAULT_FUNDING_PANEL_HEIGHT};
use crate::chart_state::ChartSurfaceId;
use crate::denomination::DisplayDenominationContext;
use crate::timeframe::Timeframe;
use iced::Color;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Chart Data Lifecycle
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            surface_id: ChartSurfaceId::Docked(id),
            timeframe: Timeframe::H1,
            clock_now_ms: current_unix_ms(),
            candles: Vec::new(),
            status: ChartStatus::Loading,
            candle_cache: canvas::Cache::new(),
            reset_epoch: 0,
            active_position: None,
            active_orders: Vec::new(),
            trade_markers: Vec::new(),
            show_trade_markers: false,
            dotted_background: false,
            dotted_background_opacity: crate::config::default_chart_dotted_background_opacity(),
            crosshair_style: Default::default(),
            crosshair_guides_enabled: true,
            crosshair_scale: crate::config::default_chart_crosshair_scale(),
            annotations: Vec::new(),
            active_tool: None,
            liquidation_buckets: Vec::new(),
            heatmap_rects: Vec::new(),
            heatmap_max_usd: 0.0,
            funding_rates: Vec::new(),
            funding_status: None,
            funding_panel_height: DEFAULT_FUNDING_PANEL_HEIGHT,
            funding_annualized: false,
            macro_indicators: crate::config::MacroIndicatorsConfig::default(),
            daily_candles: Vec::new(),
            weekly_candles: Vec::new(),
            monthly_candles: Vec::new(),
            inverted: false,
            chart_bull_color: None,
            chart_bear_color: None,
            quick_order_open: false,
            quick_order_limit_price: None,
            quick_order_line_phase: 0.0,
            order_line_phase: 0.0,
            obscure_position_prices: false,
            hide_positions_and_orders: false,
            display_denomination: DisplayDenominationContext::default(),
        }
    }

    pub(crate) fn snapshot_for_export(&self) -> Self {
        Self {
            id: self.id,
            surface_id: self.surface_id,
            timeframe: self.timeframe,
            clock_now_ms: self.clock_now_ms,
            candles: self.candles.clone(),
            status: self.status.clone(),
            candle_cache: canvas::Cache::new(),
            reset_epoch: self.reset_epoch,
            active_position: self.active_position.clone(),
            active_orders: self.active_orders.clone(),
            trade_markers: self.trade_markers.clone(),
            show_trade_markers: self.show_trade_markers,
            dotted_background: self.dotted_background,
            dotted_background_opacity: self.dotted_background_opacity,
            crosshair_style: self.crosshair_style,
            crosshair_guides_enabled: self.crosshair_guides_enabled,
            crosshair_scale: self.crosshair_scale,
            annotations: self.annotations.clone(),
            active_tool: None,
            liquidation_buckets: self.liquidation_buckets.clone(),
            heatmap_rects: self.heatmap_rects.clone(),
            heatmap_max_usd: self.heatmap_max_usd,
            funding_rates: self.funding_rates.clone(),
            funding_status: self.funding_status.clone(),
            funding_panel_height: self.funding_panel_height,
            funding_annualized: self.funding_annualized,
            macro_indicators: self.macro_indicators.clone(),
            daily_candles: self.daily_candles.clone(),
            weekly_candles: self.weekly_candles.clone(),
            monthly_candles: self.monthly_candles.clone(),
            inverted: self.inverted,
            chart_bull_color: self.chart_bull_color,
            chart_bear_color: self.chart_bear_color,
            quick_order_open: false,
            quick_order_limit_price: None,
            quick_order_line_phase: 0.0,
            order_line_phase: self.order_line_phase,
            obscure_position_prices: self.obscure_position_prices,
            hide_positions_and_orders: self.hide_positions_and_orders,
            display_denomination: self.display_denomination.clone(),
        }
    }

    pub(crate) fn clone_for_chart_id(&self, id: u64) -> Self {
        let mut chart = self.snapshot_for_export();
        chart.id = id;
        chart.surface_id = ChartSurfaceId::Docked(id);
        chart.candle_cache = canvas::Cache::new();
        chart
    }

    pub(crate) fn surface_id(&self) -> ChartSurfaceId {
        self.surface_id
    }

    pub(crate) fn set_surface_id(&mut self, surface_id: ChartSurfaceId) {
        if self.surface_id != surface_id {
            self.surface_id = surface_id;
            self.candle_cache.clear();
        }
    }

    pub fn request_view_reset(&mut self) {
        self.reset_epoch = self.reset_epoch.saturating_add(1);
        self.candle_cache.clear();
    }

    pub(crate) fn set_timeframe(&mut self, timeframe: Timeframe) {
        if self.timeframe != timeframe {
            self.timeframe = timeframe;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_clock_now_ms(&mut self, now_ms: u64) {
        self.clock_now_ms = now_ms;
    }

    pub fn set_chart_colors(&mut self, bull: Option<Color>, bear: Option<Color>) {
        if self.chart_bull_color != bull || self.chart_bear_color != bear {
            self.chart_bull_color = bull;
            self.chart_bear_color = bear;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_dotted_background(&mut self, enabled: bool, opacity: f32) {
        if self.dotted_background != enabled
            || (self.dotted_background_opacity - opacity).abs() > f32::EPSILON
        {
            self.dotted_background = enabled;
            self.dotted_background_opacity = opacity;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_crosshair_style(&mut self, style: crate::config::ChartCrosshairStyle) {
        let style = style.normalized();
        if self.crosshair_style != style {
            self.crosshair_style = style;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_crosshair_guides_enabled(&mut self, enabled: bool) {
        if self.crosshair_guides_enabled != enabled {
            self.crosshair_guides_enabled = enabled;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_crosshair_scale(&mut self, scale: f32) {
        let scale = crate::config::normalize_chart_crosshair_scale(scale);
        if (self.crosshair_scale - scale).abs() > f32::EPSILON {
            self.crosshair_scale = scale;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_display_denomination(&mut self, context: DisplayDenominationContext) {
        if self.display_denomination != context {
            self.display_denomination = context;
            self.candle_cache.clear();
        }
    }
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
