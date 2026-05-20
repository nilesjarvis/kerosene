use super::model::{
    FUNDING_MODE_BUTTON_HEIGHT, FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_X,
    FUNDING_MODE_BUTTON_Y_OFFSET,
};
use super::{
    CandlestickChart, ChartStatus, DEFAULT_FUNDING_PANEL_HEIGHT, FUNDING_PANEL_RESIZE_HIT_PX,
    MAX_FUNDING_PANEL_HEIGHT, MIN_FUNDING_PANEL_HEIGHT, MIN_MAIN_CHART_HEIGHT, TIME_AXIS_HEIGHT,
};
use crate::api::{Candle, is_valid_candle, normalize_candles};
use crate::chart_state::ChartSurfaceId;
use crate::hydromancer_api::FundingRatePoint;
use crate::timeframe::Timeframe;
use iced::Color;
use iced::widget::canvas;
use std::collections::BTreeMap;

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
        }
    }

    pub(crate) fn snapshot_for_surface(
        &self,
        surface_id: ChartSurfaceId,
        surface_reset_epoch: u64,
        active_tool: Option<crate::annotations::DrawingTool>,
        quick_order_open: bool,
        quick_order_limit_price: Option<f64>,
    ) -> Self {
        let mut snapshot = self.snapshot_for_export();
        snapshot.surface_id = surface_id;
        snapshot.reset_epoch = self.reset_epoch.saturating_add(surface_reset_epoch);
        snapshot.active_tool = active_tool;
        snapshot.quick_order_open = quick_order_open;
        snapshot.quick_order_limit_price = quick_order_limit_price;
        snapshot.quick_order_line_phase = if quick_order_open {
            self.quick_order_line_phase
        } else {
            0.0
        };
        snapshot.obscure_position_prices = self.obscure_position_prices;
        snapshot.hide_positions_and_orders = self.hide_positions_and_orders;
        snapshot
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

    pub(in crate::chart) fn chart_area_heights(&self, bounds_height: f32) -> (f32, f32) {
        if !bounds_height.is_finite() {
            return (0.0, 0.0);
        }

        let available_h = (bounds_height - TIME_AXIS_HEIGHT).max(0.0);
        let funding_h = self.funding_panel_height(available_h);
        ((available_h - funding_h).max(0.0), funding_h)
    }

    fn funding_panel_height(&self, available_h: f32) -> f32 {
        if !self.macro_indicators.show_funding_rate
            || available_h <= 0.0
            || !available_h.is_finite()
        {
            return 0.0;
        }

        let max_panel_h =
            (available_h - MIN_MAIN_CHART_HEIGHT).clamp(0.0, MAX_FUNDING_PANEL_HEIGHT);
        if max_panel_h <= 0.0 {
            return 0.0;
        }

        self.funding_panel_height
            .max(MIN_FUNDING_PANEL_HEIGHT.min(max_panel_h))
            .min(max_panel_h)
    }

    pub(crate) fn set_funding_panel_height(&mut self, height: f32) {
        let height = Self::clamp_funding_panel_height(height);
        if (self.funding_panel_height - height).abs() >= 0.5 {
            self.funding_panel_height = height;
            self.candle_cache.clear();
        }
    }

    pub(crate) fn funding_panel_height_config(&self) -> u16 {
        Self::clamp_funding_panel_height(self.funding_panel_height).round() as u16
    }

    pub(crate) fn clamp_funding_panel_height(height: f32) -> f32 {
        if height.is_finite() {
            height.clamp(MIN_FUNDING_PANEL_HEIGHT, MAX_FUNDING_PANEL_HEIGHT)
        } else {
            DEFAULT_FUNDING_PANEL_HEIGHT
        }
    }

    pub(in crate::chart) fn funding_panel_resize_target_y(
        &self,
        bounds_height: f32,
        pos_y: f32,
    ) -> Option<f32> {
        let (chart_h, funding_h) = self.chart_area_heights(bounds_height);
        if funding_h <= 0.0 {
            return None;
        }

        ((pos_y - chart_h).abs() <= FUNDING_PANEL_RESIZE_HIT_PX).then_some(chart_h)
    }

    pub(in crate::chart) fn funding_mode_button_contains(
        &self,
        bounds_height: f32,
        pos: iced::Point,
        chart_w: f32,
    ) -> bool {
        let (chart_h, funding_h) = self.chart_area_heights(bounds_height);
        if funding_h <= 0.0 || pos.x >= chart_w {
            return false;
        }

        let x = FUNDING_MODE_BUTTON_X;
        let y = chart_h + FUNDING_MODE_BUTTON_Y_OFFSET;
        pos.x >= x
            && pos.x <= x + FUNDING_MODE_BUTTON_WIDTH
            && pos.y >= y
            && pos.y <= y + FUNDING_MODE_BUTTON_HEIGHT
    }

    pub(crate) fn toggle_funding_rate_display_mode(&mut self) {
        self.funding_annualized = !self.funding_annualized;
        self.candle_cache.clear();
    }

    /// Replace all candle data (e.g. after initial fetch or interval change).
    pub fn set_candles(&mut self, candles: Vec<Candle>) {
        self.candles = normalize_candles(candles);
        self.status = if self.candles.is_empty() {
            ChartStatus::Error("No candle data returned".to_string())
        } else {
            ChartStatus::Loaded
        };
        self.candle_cache.clear();
    }

    /// Merge new candles seamlessly, preserving existing ones if applicable.
    pub fn merge_candles(&mut self, mut new_candles: Vec<Candle>) {
        new_candles = normalize_candles(new_candles);
        if self.candles.is_empty() {
            self.candles = new_candles;
        } else if !new_candles.is_empty() {
            let first_new_time = new_candles.first().map(|c| c.open_time).unwrap_or_default();

            self.candles.retain(|c| c.open_time < first_new_time);
            self.candles.append(&mut new_candles);
        }

        if self.candles.len() > 10000 {
            let trim_len = self.candles.len() - 10000;
            self.candles.drain(0..trim_len);
        }

        self.status = if self.candles.is_empty() {
            ChartStatus::Error("No candle data returned".to_string())
        } else {
            ChartStatus::Loaded
        };
        self.candle_cache.clear();
    }

    /// Append or update the latest candle from a real-time feed.
    pub fn push_candle(&mut self, candle: Candle) {
        if !is_valid_candle(&candle) {
            return;
        }
        if let Some(last) = self.candles.last_mut() {
            if last.open_time == candle.open_time {
                *last = candle;
            } else {
                self.candles.push(candle);
            }
        } else {
            self.candles.push(candle);
        }
        self.candle_cache.clear();
    }

    pub fn set_error(&mut self, msg: String) {
        self.status = ChartStatus::Error(msg);
        self.candle_cache.clear();
    }

    pub(crate) fn clear_macro_candles(&mut self) {
        self.daily_candles.clear();
        self.weekly_candles.clear();
        self.monthly_candles.clear();
        self.candle_cache.clear();
    }

    pub(crate) fn set_funding_history(&mut self, points: Vec<FundingRatePoint>) {
        self.funding_rates = normalized_funding_rates(points);
        self.funding_status = Some((
            if self.funding_rates.is_empty() {
                "Funding no data".to_string()
            } else {
                "Funding loaded".to_string()
            },
            self.funding_rates.is_empty(),
        ));
        self.candle_cache.clear();
    }

    pub(crate) fn merge_funding_history(&mut self, mut points: Vec<FundingRatePoint>) {
        if points.is_empty() {
            self.set_funding_status("Funding current".to_string(), false);
            return;
        }

        let mut merged = std::mem::take(&mut self.funding_rates);
        merged.append(&mut points);
        self.funding_rates = normalized_funding_rates(merged);
        self.funding_status = Some(("Funding loaded".to_string(), false));
        self.candle_cache.clear();
    }

    pub(crate) fn set_funding_status(&mut self, label: String, is_error: bool) {
        if self
            .funding_status
            .as_ref()
            .is_some_and(|(current, current_is_error)| {
                current == &label && *current_is_error == is_error
            })
        {
            return;
        }

        self.funding_status = Some((label, is_error));
        self.candle_cache.clear();
    }

    pub(crate) fn clear_funding_history(&mut self) {
        self.funding_rates.clear();
        self.funding_status = None;
        self.candle_cache.clear();
    }
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn normalized_funding_rates(points: Vec<FundingRatePoint>) -> Vec<FundingRatePoint> {
    let mut by_time = BTreeMap::new();
    for point in points {
        by_time.insert(point.time_ms, point);
    }
    by_time.into_values().collect()
}
