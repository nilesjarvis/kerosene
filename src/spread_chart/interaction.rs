use super::{SpreadChart, SpreadChartState};
use crate::market_state::clamp_order_book_spread_chart_height;
use crate::message::Message;
use iced::mouse;
use iced::widget::canvas::Action;
use iced::{Event, Rectangle};

#[cfg(test)]
mod tests;

const WHEEL_RESIZE_STEP: f32 = 18.0;

impl SpreadChart<'_> {
    pub(super) fn update_interaction(
        &self,
        state: &mut SpreadChartState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let pos = cursor.position_in(bounds);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(p) = pos
                    && p.y <= 10.0
                {
                    state.is_dragging = true;
                    state.hover_pos = None;
                    return Some(Action::capture());
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if pos.is_some() {
                    let new_height = wheel_resized_height(bounds.height, delta);
                    state.hover_pos = pos;
                    if (new_height - bounds.height).abs() > f32::EPSILON {
                        return Some(
                            Action::publish(Message::OrderBookSpreadChartResize(
                                self.id, new_height,
                            ))
                            .and_capture(),
                        );
                    }
                    return Some(Action::capture());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.is_dragging =>
            {
                state.is_dragging = false;
                state.hover_pos = pos;
                return Some(Action::request_redraw());
            }
            Event::Mouse(mouse::Event::CursorLeft) if !state.is_dragging => {
                state.hover_pos = None;
                return Some(Action::request_redraw());
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.is_dragging {
                    let bottom_edge = bounds.y + bounds.height;
                    let new_height = bottom_edge - position.y;
                    let clamped = clamped_height(new_height);
                    return Some(
                        Action::publish(Message::OrderBookSpreadChartResize(self.id, clamped))
                            .and_capture(),
                    );
                } else if pos.is_some() {
                    state.hover_pos = pos;
                    return Some(Action::request_redraw());
                }
            }
            _ => {}
        }

        None
    }

    pub(super) fn mouse_interaction_for(
        &self,
        state: &SpreadChartState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.is_dragging {
            return mouse::Interaction::ResizingVertically;
        }
        if let Some(p) = cursor.position_in(bounds) {
            if p.y <= 10.0 {
                return mouse::Interaction::ResizingVertically;
            }
            return mouse::Interaction::Crosshair;
        }
        mouse::Interaction::default()
    }
}

fn wheel_resized_height(current_height: f32, delta: &mouse::ScrollDelta) -> f32 {
    let lines = match delta {
        mouse::ScrollDelta::Lines { y, .. } => *y,
        mouse::ScrollDelta::Pixels { y, .. } => *y / 28.0,
    };

    clamp_order_book_spread_chart_height(current_height + lines * WHEEL_RESIZE_STEP)
}

fn clamped_height(height: f32) -> f32 {
    clamp_order_book_spread_chart_height(height)
}
