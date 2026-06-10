mod candles;
mod earnings;
mod funding;

use super::{CandlestickChart, ChartStatus, DEFAULT_FUNDING_PANEL_HEIGHT};
use crate::app_time::now_ms;
use crate::chart_state::ChartSurfaceId;
use crate::denomination::DisplayDenominationContext;
use crate::timeframe::Timeframe;
use iced::Color;
use iced::widget::canvas;
use std::collections::VecDeque;

const SPREAD_HISTORY_WINDOW_MS: u64 = 10_000;
const SPREAD_HISTORY_LIMIT: usize = 2_048;

// ---------------------------------------------------------------------------
// Chart Data Lifecycle
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            surface_id: ChartSurfaceId::Docked(id),
            symbol_label: String::new(),
            timeframe: Timeframe::H1,
            clock_now_ms: now_ms(),
            candles: Vec::new(),
            status: ChartStatus::Loading,
            candle_cache: canvas::Cache::new(),
            reset_epoch: 0,
            active_position: None,
            active_orders: Vec::new(),
            trade_markers: Vec::new(),
            earnings_markers: Vec::new(),
            show_trade_markers: false,
            dotted_background: false,
            dotted_background_opacity: crate::config::default_chart_dotted_background_opacity(),
            hollow_candle_mode: Default::default(),
            fisheye_enabled: false,
            fisheye_strength: crate::config::default_chart_fisheye_strength(),
            chromatic_aberration_enabled: false,
            chromatic_aberration_strength:
                crate::config::default_chart_chromatic_aberration_strength(),
            edge_blur_enabled: false,
            edge_blur_strength: crate::config::default_chart_edge_blur_strength(),
            crosshair_style: Default::default(),
            crosshair_guides_enabled: true,
            crosshair_scale: crate::config::default_chart_crosshair_scale(),
            hud_readout: Default::default(),
            annotations: Vec::new(),
            active_tool: None,
            liquidation_buckets: Vec::new(),
            heatmap_rects: Vec::new(),
            heatmap_max_usd: 0.0,
            funding_rates: Vec::new(),
            funding_status: None,
            funding_panel_height: DEFAULT_FUNDING_PANEL_HEIGHT,
            market_reference_price: None,
            current_spread: None,
            spread_history: VecDeque::new(),
            hud_max_notional: None,
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
            hud_order_animation: None,
            pending_market_order_loading: Vec::new(),
            hud_armed: false,
            hud_pulse_phase: 0.0,
            hud_last_activity_ms: None,
            hud_hovering: false,
            hud_idle_warning_sounded: false,
            hud_feed: Vec::new(),
            obscure_position_prices: false,
            hide_positions_and_orders: false,
            hover_order_cancel_oid: None,
            order_cancel_hover_progress: 0.0,
            hover_earnings_marker_time_ms: None,
            earnings_marker_hover_progress: 0.0,
            display_denomination: DisplayDenominationContext::default(),
        }
    }

    pub(crate) fn snapshot_for_export(&self) -> Self {
        Self {
            id: self.id,
            surface_id: self.surface_id,
            symbol_label: self.symbol_label.clone(),
            timeframe: self.timeframe,
            clock_now_ms: self.clock_now_ms,
            candles: self.candles.clone(),
            status: self.status.clone(),
            candle_cache: canvas::Cache::new(),
            reset_epoch: self.reset_epoch,
            active_position: self.active_position.clone(),
            active_orders: self.active_orders.clone(),
            trade_markers: self.trade_markers.clone(),
            earnings_markers: self.earnings_markers.clone(),
            show_trade_markers: self.show_trade_markers,
            dotted_background: self.dotted_background,
            dotted_background_opacity: self.dotted_background_opacity,
            hollow_candle_mode: self.hollow_candle_mode,
            fisheye_enabled: self.fisheye_enabled,
            fisheye_strength: self.fisheye_strength,
            chromatic_aberration_enabled: self.chromatic_aberration_enabled,
            chromatic_aberration_strength: self.chromatic_aberration_strength,
            edge_blur_enabled: self.edge_blur_enabled,
            edge_blur_strength: self.edge_blur_strength,
            crosshair_style: self.crosshair_style,
            crosshair_guides_enabled: self.crosshair_guides_enabled,
            crosshair_scale: self.crosshair_scale,
            hud_readout: self.hud_readout,
            annotations: self.annotations.clone(),
            active_tool: None,
            liquidation_buckets: self.liquidation_buckets.clone(),
            heatmap_rects: self.heatmap_rects.clone(),
            heatmap_max_usd: self.heatmap_max_usd,
            funding_rates: self.funding_rates.clone(),
            funding_status: self.funding_status.clone(),
            funding_panel_height: self.funding_panel_height,
            market_reference_price: self.market_reference_price,
            current_spread: self.current_spread,
            spread_history: self.spread_history.clone(),
            hud_max_notional: self.hud_max_notional,
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
            hud_order_animation: None,
            pending_market_order_loading: Vec::new(),
            hud_armed: false,
            hud_pulse_phase: 0.0,
            hud_last_activity_ms: None,
            hud_hovering: false,
            hud_idle_warning_sounded: false,
            hud_feed: Vec::new(),
            obscure_position_prices: self.obscure_position_prices,
            hide_positions_and_orders: self.hide_positions_and_orders,
            hover_order_cancel_oid: None,
            order_cancel_hover_progress: 0.0,
            hover_earnings_marker_time_ms: None,
            earnings_marker_hover_progress: 0.0,
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

    pub(crate) fn set_symbol_label(&mut self, symbol_label: String) {
        if self.symbol_label != symbol_label {
            self.symbol_label = symbol_label;
            // Feed rows carry no symbol; shots fired on the old instrument
            // must not linger on the new one.
            self.hud_feed.clear();
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_clock_now_ms(&mut self, now_ms: u64) {
        self.clock_now_ms = now_ms;
    }

    pub(crate) fn set_market_reference_price(&mut self, price: Option<f64>) {
        let price = price.and_then(crate::helpers::positive_finite_value);
        if self.market_reference_price != price {
            self.market_reference_price = price;
        }
    }

    /// Record the latest spread sample. A missing or invalid spread blanks the
    /// current readout but keeps the recent history so a transient gap in
    /// impact prices does not reset the racing HUD gauge baseline.
    pub(crate) fn set_current_spread_at(&mut self, spread: Option<f64>, now_ms: u64) {
        self.current_spread = spread.filter(|spread| spread.is_finite() && *spread >= 0.0);
        let Some(spread) = self.current_spread else {
            return;
        };
        self.spread_history.push_back((now_ms, spread));
        self.trim_spread_history(now_ms);
    }

    /// Drop all spread state. Used when the chart symbol changes or resets.
    pub(crate) fn clear_spread_history(&mut self) {
        self.current_spread = None;
        self.spread_history.clear();
    }

    pub(crate) fn spread_history_bounds(&self) -> Option<(f64, f64)> {
        self.spread_history
            .iter()
            .map(|(_, spread)| *spread)
            .filter(|spread| spread.is_finite() && *spread >= 0.0)
            .fold(None, |bounds, spread| {
                Some(match bounds {
                    Some((min_spread, max_spread)) => {
                        (min_spread.min(spread), max_spread.max(spread))
                    }
                    None => (spread, spread),
                })
            })
    }

    fn trim_spread_history(&mut self, now_ms: u64) {
        let cutoff_ms = now_ms.saturating_sub(SPREAD_HISTORY_WINDOW_MS);
        while self
            .spread_history
            .front()
            .is_some_and(|(sample_ms, _)| *sample_ms < cutoff_ms)
        {
            self.spread_history.pop_front();
        }
        while self.spread_history.len() > SPREAD_HISTORY_LIMIT {
            self.spread_history.pop_front();
        }
    }

    pub(crate) fn set_hud_max_notional(&mut self, max_notional: Option<f64>) {
        self.hud_max_notional = max_notional.and_then(crate::helpers::positive_finite_value);
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

    pub(crate) fn set_hollow_candle_mode(&mut self, mode: crate::config::ChartHollowCandleMode) {
        if self.hollow_candle_mode != mode {
            self.hollow_candle_mode = mode;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_fisheye(&mut self, enabled: bool, strength: f32) {
        let strength = crate::config::normalize_chart_fisheye_strength(strength);
        if self.fisheye_enabled != enabled
            || (self.fisheye_strength - strength).abs() > f32::EPSILON
        {
            self.fisheye_enabled = enabled;
            self.fisheye_strength = strength;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_chromatic_aberration(&mut self, enabled: bool, strength: f32) {
        let strength = crate::config::normalize_chart_chromatic_aberration_strength(strength);
        if self.chromatic_aberration_enabled != enabled
            || (self.chromatic_aberration_strength - strength).abs() > f32::EPSILON
        {
            self.chromatic_aberration_enabled = enabled;
            self.chromatic_aberration_strength = strength;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_edge_blur(&mut self, enabled: bool, strength: f32) {
        let strength = crate::config::normalize_chart_edge_blur_strength(strength);
        if self.edge_blur_enabled != enabled
            || (self.edge_blur_strength - strength).abs() > f32::EPSILON
        {
            self.edge_blur_enabled = enabled;
            self.edge_blur_strength = strength;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn set_crosshair_style(&mut self, style: crate::config::ChartCrosshairStyle) {
        let style = style.normalized();
        if self.crosshair_style != style {
            let was_game_hud = self.crosshair_style.is_game_hud();
            self.crosshair_style = style;
            if was_game_hud || style.is_game_hud() {
                self.clear_hud_armed();
            }
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

    pub(crate) fn set_hud_readout(&mut self, readout: crate::config::ChartHudReadoutConfig) {
        if self.hud_readout != readout {
            self.hud_readout = readout;
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

#[cfg(test)]
mod tests;
