use super::price_range::visible_price_stats;
use super::{CANDLE_GAP_RATIO, CandlestickChart, ChartViewport, DEFAULT_CANDLE_WIDTH};
use iced::Point;

// ---------------------------------------------------------------------------
// Chart Interaction State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DragKind {
    /// Dragging on the main chart area -- pans the X axis.
    PanX,
    /// Dragging on the price axis -- scales / pans the Y axis.
    PanY,
    /// Dragging on the funding panel -- pans the funding Y axis.
    PanFundingY,
    /// Dragging the top edge of the funding sub-panel.
    ResizeFundingPanel,
    /// Dragging an order line to a new price.
    MoveOrder { oid: u64 },
}

/// Widget-local mutable state for the canvas (managed by iced runtime).
#[derive(Debug)]
pub struct ChartState {
    pub(super) cursor_position: Option<Point>,
    pub(super) scroll_offset: f32,
    pub(super) candle_width: f32,
    pub(super) y_auto: bool,
    pub(super) y_offset: f64,
    pub(super) y_scale: f64,
    pub(super) funding_y_scale: f64,
    pub(super) funding_y_offset: f64,
    pub(super) drag: Option<DragKind>,
    pub(super) drag_start: Option<Point>,
    pub(super) drag_start_scroll: f32,
    pub(super) drag_start_y_offset: f64,
    pub(super) drag_start_funding_panel_height: f32,
    pub(super) drag_funding_panel_height: Option<f32>,
    /// Temporary price for an order being dragged (live preview).
    pub(super) drag_order_new_price: Option<f64>,
    /// OID of the order line the cursor is currently hovering near
    /// (used for grab cursor feedback).
    pub(super) hover_order_oid: Option<u64>,
    /// First anchor for two-click drawing tools (trend line).
    /// Stored as (timestamp_ms, price).
    pub(super) pending_anchor: Option<(u64, f64)>,
    /// True while Shift is pressed.
    pub(super) shift_down: bool,
    /// Anchor price for Shift+click range measurement.
    pub(super) range_anchor_price: Option<f64>,
    pub(super) reset_epoch_seen: u64,
}

impl Default for ChartState {
    fn default() -> Self {
        Self {
            cursor_position: None,
            scroll_offset: 0.0,
            candle_width: DEFAULT_CANDLE_WIDTH,
            y_auto: true,
            y_offset: 0.0,
            y_scale: 1.0,
            funding_y_scale: 1.0,
            funding_y_offset: 0.0,
            drag: None,
            drag_start: None,
            drag_start_scroll: 0.0,
            drag_start_y_offset: 0.0,
            drag_start_funding_panel_height: 0.0,
            drag_funding_panel_height: None,
            drag_order_new_price: None,
            hover_order_oid: None,
            pending_anchor: None,
            shift_down: false,
            range_anchor_price: None,
            reset_epoch_seen: 0,
        }
    }
}

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

    pub(super) fn reset_view(&mut self, reset_epoch: u64) {
        self.scroll_offset = 0.0;
        self.candle_width = DEFAULT_CANDLE_WIDTH;
        self.y_auto = true;
        self.y_offset = 0.0;
        self.y_scale = 1.0;
        self.funding_y_scale = 1.0;
        self.funding_y_offset = 0.0;
        self.drag = None;
        self.drag_start = None;
        self.drag_start_scroll = 0.0;
        self.drag_start_y_offset = 0.0;
        self.drag_start_funding_panel_height = 0.0;
        self.drag_funding_panel_height = None;
        self.drag_order_new_price = None;
        self.hover_order_oid = None;
        self.pending_anchor = None;
        self.range_anchor_price = None;
        self.reset_epoch_seen = reset_epoch;
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

#[cfg(test)]
mod tests {
    use super::ChartState;
    use crate::api::Candle;
    use crate::chart::{CandlestickChart, ChartViewport, PRICE_AXIS_WIDTH};

    #[test]
    fn reset_view_clears_funding_axis_transform() {
        let mut state = ChartState {
            funding_y_scale: 0.25,
            funding_y_offset: 0.001,
            ..ChartState::default()
        };

        state.reset_view(42);

        assert_eq!(state.funding_y_scale, 1.0);
        assert_eq!(state.funding_y_offset, 0.0);
        assert_eq!(state.reset_epoch_seen, 42);
    }

    #[test]
    fn export_state_reconstructs_time_viewport() {
        let chart = test_chart();
        let chart_w = 800.0 - PRICE_AXIS_WIDTH;
        let viewport = ChartViewport {
            start_time_ms: 60_000,
            end_time_ms: 240_000,
            price_lo: 90.0,
            price_hi: 130.0,
            chart_width: 0.0,
            candle_width: 0.0,
            scroll_offset: 0.0,
            y_auto: true,
            y_scale: 1.0,
            y_offset: 0.0,
            funding_y_scale: 1.0,
            funding_y_offset: 0.0,
        };

        let state = ChartState::for_export_viewport(&chart, Some(viewport), chart_w);
        let left = chart.x_to_timestamp(0.0, &state, chart_w).expect("left ts");
        let right = chart
            .x_to_timestamp(chart_w, &state, chart_w)
            .expect("right ts");

        assert_eq!(left, viewport.start_time_ms);
        assert_eq!(right, viewport.end_time_ms);
    }

    #[test]
    fn export_state_reconstructs_price_viewport() {
        let chart = test_chart();
        let chart_w = 800.0 - PRICE_AXIS_WIDTH;
        let viewport = ChartViewport {
            start_time_ms: 60_000,
            end_time_ms: 240_000,
            price_lo: 90.0,
            price_hi: 130.0,
            chart_width: 0.0,
            candle_width: 0.0,
            scroll_offset: 0.0,
            y_auto: true,
            y_scale: 1.0,
            y_offset: 0.0,
            funding_y_scale: 0.5,
            funding_y_offset: 0.002,
        };

        let state = ChartState::for_export_viewport(&chart, Some(viewport), chart_w);
        let Some((price_hi, price_range, _)) = chart.visible_price_params(&state, chart_w, 500.0)
        else {
            panic!("price params");
        };

        assert!((price_hi - viewport.price_hi).abs() < 0.0001);
        assert!((price_range - (viewport.price_hi - viewport.price_lo)).abs() < 0.0001);
        assert_eq!(state.funding_y_scale, viewport.funding_y_scale);
        assert_eq!(state.funding_y_offset, viewport.funding_y_offset);
    }

    #[test]
    fn export_state_preserves_right_empty_space_from_geometry() {
        let chart = test_chart();
        let source_chart_w = 500.0;
        let target_chart_w = 1000.0;
        let viewport = ChartViewport {
            start_time_ms: 180_000,
            end_time_ms: 420_000,
            price_lo: 90.0,
            price_hi: 130.0,
            chart_width: source_chart_w,
            candle_width: 20.0,
            scroll_offset: -4.0,
            y_auto: false,
            y_scale: 1.75,
            y_offset: 2.0,
            funding_y_scale: 0.8,
            funding_y_offset: -0.001,
        };

        let state = ChartState::for_export_viewport(&chart, Some(viewport), target_chart_w);

        assert_eq!(state.scroll_offset, viewport.scroll_offset);
        assert_eq!(state.candle_width, 40.0);
        assert!(!state.y_auto);
        assert_eq!(state.y_scale, viewport.y_scale);
        assert_eq!(state.y_offset, viewport.y_offset);

        let last_x = chart
            .timestamp_to_x(
                chart.candles.last().expect("last candle").open_time,
                &state,
                target_chart_w,
            )
            .expect("last candle x");
        let step = state.candle_width * (1.0 + crate::chart::CANDLE_GAP_RATIO);

        assert!((target_chart_w - last_x - step * 4.5).abs() < 0.0001);
    }

    fn test_chart() -> CandlestickChart {
        let mut chart = CandlestickChart::new(1);
        chart.candles = (0..8)
            .map(|idx| Candle {
                open_time: idx * 60_000,
                close_time: idx * 60_000 + 59_999,
                open: 100.0 + idx as f64,
                high: 110.0 + idx as f64,
                low: 95.0 + idx as f64,
                close: 104.0 + idx as f64,
                volume: 1000.0 + idx as f64,
            })
            .collect();
        chart
    }
}
