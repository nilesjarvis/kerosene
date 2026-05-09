use crate::chart::{
    CandlestickChart, ChartState, ChartViewport, PRICE_AXIS_WIDTH, TIME_AXIS_HEIGHT,
};
use crate::message::Message;
use iced::Rectangle;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Viewport Actions
// ---------------------------------------------------------------------------

impl CandlestickChart {
    fn current_viewport(
        &self,
        state: &ChartState,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<ChartViewport> {
        if self.candles.is_empty() || chart_w <= 0.0 || chart_h <= 0.0 {
            return None;
        }

        let (price_hi, price_range, _) = self.visible_price_params(state, chart_w, chart_h)?;
        let left_ts = self.x_to_timestamp(0.0, state, chart_w)?;
        let right_ts = self.x_to_timestamp(chart_w, state, chart_w)?;
        let first_ts = self.candles.first()?.open_time;
        let last_ts = self.candles.last()?.open_time;
        let start_time_ms = left_ts.min(right_ts).clamp(first_ts, last_ts);
        let end_time_ms = left_ts.max(right_ts).clamp(first_ts, last_ts);

        if end_time_ms <= start_time_ms {
            return None;
        }

        Some(ChartViewport {
            start_time_ms,
            end_time_ms,
            price_lo: price_hi - price_range,
            price_hi,
        })
    }

    pub(in crate::chart) fn viewport_action(
        &self,
        state: &ChartState,
        bounds: Rectangle,
    ) -> Option<canvas::Action<Message>> {
        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let chart_h = bounds.height - TIME_AXIS_HEIGHT;
        self.current_viewport(state, chart_w, chart_h)
            .map(|viewport| {
                canvas::Action::publish(Message::ChartViewportChanged(self.id, viewport))
                    .and_capture()
            })
    }
}
