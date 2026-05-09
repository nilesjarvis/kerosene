use super::super::state::DragKind;
use super::super::{SpaghettiCanvas, SpaghettiChartState};
use crate::message::Message;

use iced::Point;
use iced::widget::canvas;

impl SpaghettiCanvas {
    pub(super) fn begin_left_drag(
        &self,
        state: &mut SpaghettiChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) {
        if pos.x >= chart_w && pos.y < chart_h {
            state.drag = Some(DragKind::PanY);
            state.drag_start = Some(pos);
            state.drag_start_y_offset = state.y_offset;
            if state.y_auto {
                state.y_auto = false;
                state.y_offset = 0.0;
                state.y_scale = 1.0;
                state.drag_start_y_offset = 0.0;
            }
        } else if pos.x < chart_w && pos.y < chart_h {
            state.drag = Some(DragKind::PanX);
            state.drag_start = Some(pos);
            state.drag_start_scroll = state.scroll_offset_ms;
        }
    }

    pub(super) fn handle_drag_move(
        &self,
        state: &mut SpaghettiChartState,
        pos: Option<Point>,
        chart_w: f32,
        chart_h: f32,
        horizontal_px_per_ms: f64,
        max_scroll: f64,
    ) -> Option<canvas::Action<Message>> {
        let (Some(kind), Some(start), Some(pos)) = (state.drag, state.drag_start, pos) else {
            return None;
        };

        match kind {
            DragKind::PanX => {
                let dx = pos.x - start.x;
                let delta_ms = dx as f64 / horizontal_px_per_ms;
                state.scroll_offset_ms = (state.drag_start_scroll - delta_ms)
                    .clamp(self.min_scroll_for(chart_w, state.px_per_ms), max_scroll);
            }
            DragKind::PanY => {
                let base_range = 20.0;
                let pct_per_px = base_range / chart_h as f64;
                state.y_offset = state.drag_start_y_offset + (pos.y - start.y) as f64 * pct_per_px;
            }
        }

        self.cache.clear();
        Some(canvas::Action::request_redraw())
    }

    pub(super) fn finish_left_drag(
        &self,
        state: &mut SpaghettiChartState,
    ) -> Option<canvas::Action<Message>> {
        state.drag?;

        state.drag = None;
        state.drag_start = None;
        Some(canvas::Action::request_redraw())
    }

    pub(super) fn reset_y_axis_at(
        &self,
        state: &mut SpaghettiChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<canvas::Action<Message>> {
        if pos.x < chart_w || pos.y >= chart_h {
            return None;
        }

        state.y_auto = true;
        state.y_offset = 0.0;
        state.y_scale = 1.0;
        self.cache.clear();
        Some(canvas::Action::request_redraw())
    }
}
