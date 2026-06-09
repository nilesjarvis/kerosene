mod cursor;
mod drag;
mod drawing;
mod hud;
mod press;
mod zoom;

use super::{CandlestickChart, ChartState, VOLUME_REGION_RATIO};
use crate::chart::fisheye::ChartFisheye;
use crate::message::Message;
use iced::Point;
use iced::Rectangle;
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Chart Interaction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(in crate::chart) struct ProjectedCursor {
    pub(in crate::chart) source: Point,
    pub(in crate::chart) visual: Point,
}

impl ProjectedCursor {
    #[cfg(test)]
    pub(in crate::chart) fn identity(point: Point) -> Self {
        Self {
            source: point,
            visual: point,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::chart) struct InteractionLayout {
    pub(in crate::chart) chart_w: f32,
    pub(in crate::chart) chart_h: f32,
    pub(in crate::chart) funding_panel_h: f32,
}

impl InteractionLayout {
    #[cfg(test)]
    pub(in crate::chart) fn without_funding(chart_w: f32, chart_h: f32) -> Self {
        Self {
            chart_w,
            chart_h,
            funding_panel_h: 0.0,
        }
    }
}

impl CandlestickChart {
    pub(super) fn update_interaction(
        &self,
        state: &mut ChartState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let raw_pos = cursor.position_in(bounds);
        let chart_w = bounds.width - self.price_axis_width();
        let (chart_h, funding_panel_h) = self.chart_area_heights(bounds.height);
        let layout = InteractionLayout {
            chart_w,
            chart_h,
            funding_panel_h,
        };

        if chart_w <= 0.0
            || chart_h <= 0.0
            || !chart_w.is_finite()
            || !chart_h.is_finite()
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
        {
            let needs_redraw_for_hover = state.hover_order_oid.take().is_some();
            let needs_redraw_for_cursor = state.cursor_position != raw_pos;
            state.cursor_position = raw_pos;
            return (needs_redraw_for_cursor || needs_redraw_for_hover)
                .then(canvas::Action::request_redraw);
        }

        let fisheye = ChartFisheye::new(
            self.fisheye_enabled,
            self.fisheye_strength,
            chart_w,
            chart_h + funding_panel_h,
        );
        let projected_cursor = raw_pos.map(|visual| ProjectedCursor {
            source: fisheye.unproject(visual),
            visual,
        });
        let source_pos = projected_cursor.map(|cursor| cursor.source);
        let needs_redraw_for_cursor = state.cursor_position != source_pos;
        state.cursor_position = source_pos;

        if state.reset_epoch_seen != self.reset_epoch {
            state.reset_view(self.reset_epoch);
            self.candle_cache.clear();
            if let Some(action) = self.viewport_action(state, bounds) {
                return Some(action);
            }
            return Some(canvas::Action::request_redraw());
        }

        match event {
            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
                let shift_next = mods.shift();
                let ctrl_next = mods.control();
                if state.shift_down != shift_next || state.ctrl_down != ctrl_next {
                    state.shift_down = shift_next;
                    state.ctrl_down = ctrl_next;
                    if !ctrl_next {
                        state.hud_size_scroll_bias = 0.0;
                    }
                    return Some(canvas::Action::request_redraw());
                }
                None
            }
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) => self.handle_hud_key_pressed(state, key.as_ref(), text.as_deref(), *modifiers),
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let cursor = projected_cursor?;
                self.handle_wheel_scroll(state, bounds, cursor.source, chart_w, chart_h, delta)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let cursor = projected_cursor?;
                self.handle_left_press_at(state, cursor, fisheye, layout, bounds.height)
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => {
                needs_redraw_for_cursor.then(canvas::Action::request_redraw)
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let was_dragging = state.drag.is_some();
                let hovering_plot =
                    source_pos.is_some_and(|pos| pos.x < chart_w && pos.y < chart_h);
                let order_cancel_hover_oid = if was_dragging {
                    None
                } else {
                    projected_cursor.and_then(|cursor| {
                        self.hit_test_order_cancel_at(
                            state,
                            cursor.source,
                            cursor.visual,
                            chart_w,
                            chart_h,
                            fisheye,
                        )
                    })
                };
                let price_h = chart_h * (1.0 - VOLUME_REGION_RATIO);
                let earnings_marker_time_ms = if was_dragging {
                    None
                } else {
                    projected_cursor.and_then(|cursor| {
                        self.hit_test_earnings_marker_at(state, cursor.source, chart_w, price_h)
                    })
                };
                let action = self.handle_cursor_moved(
                    state,
                    projected_cursor,
                    fisheye,
                    layout,
                    needs_redraw_for_cursor,
                );
                if let Some(hover_action) = self.hover_state_action(
                    order_cancel_hover_oid,
                    hovering_plot,
                    earnings_marker_time_ms,
                ) {
                    Some(hover_action)
                } else {
                    action
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.handle_left_release(state, bounds)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let cursor = projected_cursor?;
                self.handle_right_press_at(state, bounds, cursor, fisheye, layout)
            }
            _ => {
                if needs_redraw_for_cursor {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
        }
    }
}
