use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartState};

// ---------------------------------------------------------------------------
// Viewport Scroll Bounds
// ---------------------------------------------------------------------------

impl CandlestickChart {
    fn scroll_bounds_for(&self, chart_w: f32, candle_width: f32) -> (f32, f32) {
        if self.candles.is_empty()
            || chart_w <= 0.0
            || !chart_w.is_finite()
            || candle_width <= 0.0
            || !candle_width.is_finite()
        {
            return (0.0, 0.0);
        }

        let step = candle_width * (1.0 + CANDLE_GAP_RATIO);
        let visible_slots = (chart_w / step).max(1.0);
        let min_scroll = -(visible_slots * 0.8);
        let max_scroll = self.candles.len().saturating_sub(1) as f32;
        (min_scroll, max_scroll)
    }

    pub(in crate::chart) fn clamp_scroll_offset_for(
        &self,
        scroll_offset: f32,
        chart_w: f32,
        candle_width: f32,
    ) -> f32 {
        let (min_scroll, max_scroll) = self.scroll_bounds_for(chart_w, candle_width);
        if scroll_offset.is_finite() {
            scroll_offset.clamp(min_scroll, max_scroll)
        } else {
            0.0
        }
    }

    pub(super) fn effective_scroll_offset(&self, state: &ChartState, chart_w: f32) -> f32 {
        self.clamp_scroll_offset_for(state.scroll_offset, chart_w, state.candle_width)
    }
}
