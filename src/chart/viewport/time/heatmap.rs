use crate::chart::{CandlestickChart, ChartState};
use crate::hyperdash_api::HeatmapRect;

// ---------------------------------------------------------------------------
// Heatmap Time Bounds
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(in crate::chart) fn heatmap_x_bounds(
        &self,
        rect: &HeatmapRect,
        state: &ChartState,
        chart_w: f32,
        fallback_step: f32,
    ) -> Option<(f32, f32)> {
        let start_x = self.timestamp_to_x(rect.timestamp_ms, state, chart_w)?;
        let end_ts = rect.timestamp_ms.saturating_add(rect.duration_ms);
        let end_x = self
            .timestamp_to_x(end_ts, state, chart_w)
            .unwrap_or(start_x + fallback_step);
        let left = start_x.min(end_x);
        let right = start_x.max(end_x);

        if right <= left {
            Some((left, left + fallback_step.max(1.0)))
        } else {
            Some((left, right))
        }
    }
}
