use super::helpers::{anchored_max_scroll_offset, anchored_time_px_per_ms, global_time_range};
use super::state::DragKind;
use super::{PRICE_AXIS_WIDTH, SpaghettiCanvas, SpaghettiChartState, TIME_AXIS_HEIGHT};
use crate::message::Message;
use iced::Rectangle;
use iced::mouse;
use iced::widget::canvas;

mod drag;
mod reset;
mod rules;
mod wheel;

use rules::minimum_scroll_offset;

// ---------------------------------------------------------------------------
// Canvas Interaction
// ---------------------------------------------------------------------------

impl SpaghettiCanvas {
    pub(super) fn update_interaction(
        &self,
        state: &mut SpaghettiChartState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.position_in(bounds);
        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let chart_h = bounds.height - TIME_AXIS_HEIGHT;
        let needs_redraw = state.cursor_position != pos;
        state.cursor_position = pos;

        let loaded = self.loaded_series();

        if state.reset_epoch_seen != self.reset_epoch {
            let reset_px_per_ms = self.reset_px_per_ms(chart_w, &loaded);
            state.reset_view_with_px(self.reset_epoch, reset_px_per_ms);
            self.cache.clear();
            return Some(canvas::Action::request_redraw());
        }

        let pair_latest_ratio = self.pair_latest_ratio_for(&loaded);
        let (effective_max, unanchored_max_scroll) =
            if let Some((min, max)) = global_time_range(&loaded) {
                let effective_max = if min == max { max + 3_600_000 } else { max };
                (effective_max, (max - min) as f64)
            } else {
                (0, 0.0)
            };
        let max_scroll = self.max_scroll_for(
            chart_w,
            state.px_per_ms,
            unanchored_max_scroll,
            effective_max,
        );
        let horizontal_px_per_ms =
            self.horizontal_time_px_per_ms(chart_w, state.px_per_ms, effective_max);

        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => self.handle_wheel_scroll(
                state,
                delta,
                pos?,
                wheel::WheelScrollContext {
                    chart_w,
                    chart_h,
                    unanchored_max_scroll,
                    effective_max,
                    pair_latest_ratio,
                },
            ),
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                self.begin_left_drag(state, pos?, chart_w, chart_h);
                None
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(action) = self.handle_drag_move(
                    state,
                    pos,
                    chart_w,
                    chart_h,
                    horizontal_px_per_ms,
                    max_scroll,
                ) {
                    return Some(action);
                }
                if needs_redraw {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.finish_left_drag(state)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                self.reset_y_axis_at(state, pos?, chart_w, chart_h)
            }
            _ => {
                if needs_redraw {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn mouse_interaction_for(
        &self,
        state: &SpaghettiChartState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(pos) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let chart_h = bounds.height - TIME_AXIS_HEIGHT;
        match state.drag {
            Some(DragKind::PanX) => mouse::Interaction::Grabbing,
            Some(DragKind::PanY) => mouse::Interaction::ResizingVertically,
            None => {
                if pos.x >= chart_w && pos.y < chart_h {
                    mouse::Interaction::ResizingVertically
                } else if pos.x < chart_w && pos.y < chart_h {
                    mouse::Interaction::Crosshair
                } else {
                    mouse::Interaction::default()
                }
            }
        }
    }

    fn min_scroll_for(&self, chart_w: f32, px_per_ms: f64) -> f64 {
        minimum_scroll_offset(
            chart_w,
            px_per_ms,
            self.pair_ratio_mode,
            self.active_session.is_some(),
        )
    }

    pub(super) fn max_scroll_for(
        &self,
        chart_w: f32,
        px_per_ms: f64,
        unanchored_max_scroll: f64,
        effective_max: u64,
    ) -> f64 {
        if let Some(base_ts) = self.base_timestamp
            && self.active_session.is_some()
        {
            return anchored_max_scroll_offset(effective_max, base_ts, px_per_ms, chart_w);
        }

        unanchored_max_scroll
    }

    pub(super) fn horizontal_time_px_per_ms(
        &self,
        chart_w: f32,
        px_per_ms: f64,
        effective_max: u64,
    ) -> f64 {
        if let Some(base_ts) = self.base_timestamp
            && self.active_session.is_some()
        {
            return anchored_time_px_per_ms(effective_max, base_ts, px_per_ms, chart_w);
        }

        px_per_ms
    }
}
