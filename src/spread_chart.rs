use iced::mouse;
use iced::widget::canvas::{self, Action};
use iced::{Event, Point, Rectangle, Renderer, Theme};

use crate::message::Message;

mod interaction;
mod rendering;

#[derive(Default)]
pub struct SpreadChartState {
    pub is_dragging: bool,
    pub hover_pos: Option<Point>,
}

pub struct SpreadChart<'a> {
    pub id: u64,
    pub data: &'a std::collections::VecDeque<(std::time::Instant, f64)>,
    pub spread_decimals: usize,
}

impl<'a> canvas::Program<Message> for SpreadChart<'a> {
    type State = SpreadChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        self.update_interaction(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        self.draw_chart(state, renderer, theme, bounds)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        self.mouse_interaction_for(state, bounds, cursor)
    }
}
