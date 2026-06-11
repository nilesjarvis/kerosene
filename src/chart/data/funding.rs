use super::super::model::{
    CandlestickChart, FUNDING_MODE_BUTTON_HEIGHT, FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_X,
    FUNDING_MODE_BUTTON_Y_OFFSET,
};
use super::super::{
    DEFAULT_FUNDING_PANEL_HEIGHT, DEFAULT_SESSION_PANEL_HEIGHT, FUNDING_PANEL_RESIZE_HIT_PX,
    MAX_FUNDING_PANEL_HEIGHT, MAX_SESSION_PANEL_HEIGHT, MIN_FUNDING_PANEL_HEIGHT,
    MIN_MAIN_CHART_HEIGHT, MIN_SESSION_PANEL_HEIGHT, TIME_AXIS_HEIGHT,
};
use crate::hydromancer_api::FundingRatePoint;
use crate::timeframe::Timeframe;

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Funding Data Lifecycle
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart) fn chart_area_heights(&self, bounds_height: f32) -> (f32, f32, f32) {
        if !bounds_height.is_finite() {
            return (0.0, 0.0, 0.0);
        }

        let available_h = (bounds_height - TIME_AXIS_HEIGHT).max(0.0);
        let session_h = self.session_panel_height(available_h);
        let funding_h = self.funding_panel_height((available_h - session_h).max(0.0));
        (
            (available_h - funding_h - session_h).max(0.0),
            funding_h,
            session_h,
        )
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

    fn session_panel_height(&self, available_h: f32) -> f32 {
        if !self.session_indicator_visible() || available_h <= 0.0 || !available_h.is_finite() {
            return 0.0;
        }

        let max_panel_h =
            (available_h - MIN_MAIN_CHART_HEIGHT).clamp(0.0, MAX_SESSION_PANEL_HEIGHT);
        if max_panel_h <= 0.0 {
            return 0.0;
        }

        DEFAULT_SESSION_PANEL_HEIGHT
            .max(MIN_SESSION_PANEL_HEIGHT.min(max_panel_h))
            .min(max_panel_h)
    }

    fn session_indicator_visible(&self) -> bool {
        self.macro_indicators.show_session_indicator
            && self.timeframe.duration_ms() < Timeframe::D1.duration_ms()
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
        let (chart_h, funding_h, _) = self.chart_area_heights(bounds_height);
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
        let (chart_h, funding_h, _) = self.chart_area_heights(bounds_height);
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

fn normalized_funding_rates(points: Vec<FundingRatePoint>) -> Vec<FundingRatePoint> {
    let mut by_time = BTreeMap::new();
    for point in points {
        by_time.insert(point.time_ms, point);
    }
    by_time.into_values().collect()
}
