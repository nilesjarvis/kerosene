use super::super::{
    CANDLE_GAP_RATIO, CandlestickChart, ChartState, MAX_CANDLE_WIDTH, MIN_CANDLE_WIDTH, ZOOM_SPEED,
};
use crate::message::Message;
use iced::mouse;
use iced::widget::canvas;
use iced::{Point, Rectangle};

// ---------------------------------------------------------------------------
// Wheel Zoom Handling
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn handle_wheel_scroll(
        &self,
        state: &mut ChartState,
        bounds: Rectangle,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
        delta: &mouse::ScrollDelta,
    ) -> Option<canvas::Action<Message>> {
        let dy = match delta {
            mouse::ScrollDelta::Lines { y, .. } => *y,
            mouse::ScrollDelta::Pixels { y, .. } => *y / 28.0,
        };
        if dy == 0.0 {
            return None;
        }

        if self.hud_game_mode_enabled() && state.ctrl_down && pos.x < chart_w && pos.y < chart_h {
            return self.handle_hud_size_scroll(state, dy);
        }

        let (_, funding_panel_h) = self.chart_area_heights(bounds.height);
        let funding_axis_hover = funding_panel_h > 0.0
            && pos.x >= chart_w
            && pos.y >= chart_h
            && pos.y < chart_h + funding_panel_h;

        if funding_axis_hover {
            let factor = if dy > 0.0 {
                1.0 / ZOOM_SPEED as f64
            } else {
                ZOOM_SPEED as f64
            };
            state.funding_y_scale = (state.funding_y_scale * factor).clamp(0.1, 20.0);
        } else if pos.x >= chart_w && pos.x <= bounds.width && pos.y < chart_h {
            let factor = if dy > 0.0 {
                1.0 / ZOOM_SPEED as f64
            } else {
                ZOOM_SPEED as f64
            };
            state.y_scale = (state.y_scale * factor).clamp(0.1, 20.0);
            state.y_auto = false;
        } else if pos.x < chart_w {
            let old_w = state.candle_width;
            let factor = if dy > 0.0 {
                ZOOM_SPEED
            } else {
                1.0 / ZOOM_SPEED
            };
            let new_w = (old_w * factor).clamp(MIN_CANDLE_WIDTH, MAX_CANDLE_WIDTH);
            let step_old = old_w * (1.0 + CANDLE_GAP_RATIO);
            let step_new = new_w * (1.0 + CANDLE_GAP_RATIO);

            let candles_right_of_cursor = (chart_w - pos.x) / step_old + state.scroll_offset;
            let new_offset = candles_right_of_cursor - (chart_w - pos.x) / step_new;
            state.candle_width = new_w;
            state.scroll_offset = self.clamp_scroll_offset_for(new_offset, chart_w, new_w);
        }
        self.candle_cache.clear();
        if let Some(action) = self.viewport_action(state, bounds) {
            Some(action)
        } else {
            Some(canvas::Action::request_redraw())
        }
    }
}
