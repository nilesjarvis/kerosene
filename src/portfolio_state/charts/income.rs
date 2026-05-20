mod rendering;
mod series;

use self::rendering::draw_income_projection_chart;

use crate::denomination::DisplayDenominationContext;
use crate::message::Message;

use iced::widget::canvas;
use iced::{Point, Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Income Projection Chart
// ---------------------------------------------------------------------------

pub(crate) struct IncomeProjectionChart {
    pub(crate) bars: Vec<(String, f64)>,
    pub(crate) denomination: DisplayDenominationContext,
}

#[derive(Default)]
pub struct IncomeProjectionState {
    cursor_position: Option<Point>,
}

impl canvas::Program<Message> for IncomeProjectionChart {
    type State = IncomeProjectionState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let pos = cursor.position_in(bounds);
        let changed = state.cursor_position != pos;
        state.cursor_position = pos;

        match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if changed {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_income_projection_chart(
            &self.bars,
            &self.denomination,
            renderer,
            theme,
            bounds,
            cursor,
        )
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if cursor.position_in(bounds).is_some() {
            iced::mouse::Interaction::Crosshair
        } else {
            iced::mouse::Interaction::default()
        }
    }
}
