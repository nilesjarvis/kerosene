use super::geometry::hover_target;
use super::{DepthChart, DepthChartState};
use crate::message::Message;
use iced::mouse;
use iced::widget::canvas::Action;
use iced::{Event, Rectangle};

impl DepthChart {
    pub(super) fn update_interaction(
        &self,
        state: &mut DepthChartState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let pos = cursor.position_in(bounds);
                if state.hover_pos != pos {
                    state.hover_pos = pos;
                    return Some(Action::request_redraw());
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) if state.hover_pos.is_some() => {
                state.hover_pos = None;
                return Some(Action::request_redraw());
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds)
                    && let Some(layout) = self.layout(bounds)
                    && let Some(target) = hover_target(&self.bids, &self.asks, &layout, pos.x)
                {
                    return Some(
                        Action::publish(Message::OrderBookPriceSelected {
                            id: self.id,
                            price: format!("{:.decimals$}", target.price, decimals = self.decimals)
                                .into(),
                        })
                        .and_capture(),
                    );
                }
            }
            _ => {}
        }

        None
    }

    pub(super) fn mouse_interaction_for(
        &self,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.position_in(bounds).is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}
