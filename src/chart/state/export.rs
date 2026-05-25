use super::ChartState;
use crate::chart::price_range::visible_price_stats;
use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartViewport};

// ---------------------------------------------------------------------------
// Export Viewport Reconstruction
// ---------------------------------------------------------------------------

impl ChartState {
    pub(crate) fn for_export_viewport(
        chart: &CandlestickChart,
        viewport: Option<ChartViewport>,
        chart_w: f32,
    ) -> Self {
        let mut state = Self::default();
        let Some(viewport) = viewport else {
            return state;
        };

        if chart_w <= 0.0
            || !chart_w.is_finite()
            || viewport.end_time_ms <= viewport.start_time_ms
            || viewport.price_hi <= viewport.price_lo
            || !viewport.price_hi.is_finite()
            || !viewport.price_lo.is_finite()
        {
            return state;
        }

        if apply_export_geometry_viewport(&mut state, viewport, chart_w) {
            return state;
        }

        let Some(start_idx) = fractional_candle_index(chart, viewport.start_time_ms) else {
            return state;
        };
        let Some(end_idx) = fractional_candle_index(chart, viewport.end_time_ms) else {
            return state;
        };
        let visible_slots = end_idx - start_idx;
        if visible_slots <= 0.0 || !visible_slots.is_finite() {
            return state;
        }

        let step = (chart_w as f64 / visible_slots) as f32;
        let candle_width = step / (1.0 + CANDLE_GAP_RATIO);
        if candle_width <= 0.0 || !candle_width.is_finite() {
            return state;
        }

        let last_idx = chart.candles.len().saturating_sub(1) as f64;
        let right_idx = end_idx - 0.5;
        state.candle_width = candle_width;
        state.scroll_offset = (last_idx - right_idx) as f32;

        if let Some(range) = chart.visible_candle_range(&state, chart_w)
            && let Some(auto_stats) =
                visible_price_stats(&chart.candles[range.first..=range.last], true, 1.0, 0.0)
        {
            let auto_range = auto_stats.price_range;
            let viewport_range = viewport.price_hi - viewport.price_lo;
            if auto_range > 0.0 && viewport_range > 0.0 {
                let auto_mid = (auto_stats.price_hi + auto_stats.price_lo) * 0.5;
                let viewport_mid = (viewport.price_hi + viewport.price_lo) * 0.5;
                state.y_auto = false;
                state.y_scale = viewport_range / auto_range;
                state.y_offset = viewport_mid - auto_mid;
            }
        }
        apply_export_funding_viewport(&mut state, viewport);

        state
    }
}

fn apply_export_geometry_viewport(
    state: &mut ChartState,
    viewport: ChartViewport,
    chart_w: f32,
) -> bool {
    if viewport.chart_width <= 0.0
        || !viewport.chart_width.is_finite()
        || viewport.candle_width <= 0.0
        || !viewport.candle_width.is_finite()
        || !viewport.scroll_offset.is_finite()
    {
        return false;
    }

    let scale = chart_w / viewport.chart_width;
    if scale <= 0.0 || !scale.is_finite() {
        return false;
    }

    state.candle_width = viewport.candle_width * scale;
    state.scroll_offset = viewport.scroll_offset;
    if state.candle_width <= 0.0 || !state.candle_width.is_finite() {
        return false;
    }

    state.y_auto = viewport.y_auto;
    if viewport.y_scale > 0.0 && viewport.y_scale.is_finite() {
        state.y_scale = viewport.y_scale;
    }
    if viewport.y_offset.is_finite() {
        state.y_offset = viewport.y_offset;
    }
    apply_export_funding_viewport(state, viewport);

    true
}

fn apply_export_funding_viewport(state: &mut ChartState, viewport: ChartViewport) {
    if viewport.funding_y_scale > 0.0 && viewport.funding_y_scale.is_finite() {
        state.funding_y_scale = viewport.funding_y_scale;
    }
    if viewport.funding_y_offset.is_finite() {
        state.funding_y_offset = viewport.funding_y_offset;
    }
}

fn fractional_candle_index(chart: &CandlestickChart, timestamp_ms: u64) -> Option<f64> {
    if chart.candles.is_empty() {
        return None;
    }

    match chart
        .candles
        .binary_search_by_key(&timestamp_ms, |candle| candle.open_time)
    {
        Ok(index) => Some(index as f64),
        Err(0) => {
            let t0 = chart.candles[0].open_time as f64;
            let t1 = chart
                .candles
                .get(1)
                .map(|candle| candle.open_time as f64)
                .unwrap_or(t0 + 60_000.0);
            let dt = t1 - t0;
            if dt > 0.0 {
                Some((timestamp_ms as f64 - t0) / dt)
            } else {
                Some(0.0)
            }
        }
        Err(index) if index >= chart.candles.len() => {
            let last_idx = chart.candles.len() - 1;
            let t1 = chart.candles[last_idx].open_time as f64;
            let t0 = if last_idx > 0 {
                chart.candles[last_idx - 1].open_time as f64
            } else {
                t1 - 60_000.0
            };
            let dt = t1 - t0;
            if dt > 0.0 {
                Some(last_idx as f64 + (timestamp_ms as f64 - t1) / dt)
            } else {
                Some(last_idx as f64)
            }
        }
        Err(index) => {
            let t0 = chart.candles[index - 1].open_time as f64;
            let t1 = chart.candles[index].open_time as f64;
            let dt = t1 - t0;
            if dt > 0.0 {
                Some((index - 1) as f64 + (timestamp_ms as f64 - t0) / dt)
            } else {
                Some(index as f64)
            }
        }
    }
}
