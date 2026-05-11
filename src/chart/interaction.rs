mod cursor;
mod drag;
mod drawing;
mod press;
mod zoom;

use super::{CandlestickChart, ChartState, PRICE_AXIS_WIDTH};
use crate::message::Message;
use iced::Rectangle;
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// Chart Interaction
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn update_interaction(
        &self,
        state: &mut ChartState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.position_in(bounds);
        let chart_w = bounds.width - PRICE_AXIS_WIDTH;
        let (chart_h, funding_panel_h) = self.chart_area_heights(bounds.height);
        let needs_redraw_for_cursor = state.cursor_position != pos;
        state.cursor_position = pos;

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
                let next = mods.shift();
                if state.shift_down != next {
                    state.shift_down = next;
                    return Some(canvas::Action::request_redraw());
                }
                None
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let pos = pos?;
                self.handle_wheel_scroll(state, bounds, pos, chart_w, chart_h, delta)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = pos?;
                self.handle_left_press(state, pos, chart_w, chart_h, bounds.height)
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => self.handle_cursor_moved(
                state,
                pos,
                chart_w,
                chart_h,
                funding_panel_h,
                needs_redraw_for_cursor,
            ),
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.handle_left_release(state, bounds)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let pos = pos?;
                self.handle_right_press(state, bounds, pos, chart_w, chart_h)
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
