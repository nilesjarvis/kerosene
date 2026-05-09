use super::{SpreadChart, SpreadChartState};
use crate::message::Message;
use iced::mouse;
use iced::widget::canvas::Action;
use iced::{Event, Rectangle};

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
                    let clamped = if new_height.is_finite() {
                        new_height.clamp(30.0, 1000.0)
                    } else {
                        30.0
                    };
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
