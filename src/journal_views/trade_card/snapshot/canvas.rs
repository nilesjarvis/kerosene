use super::drawing::draw_snapshot_chart;
use super::interaction::{JournalSnapshotCanvasState, update_snapshot_interaction};
use crate::journal::JournalTradeSnapshot;
use crate::message::Message;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Theme, mouse};

#[derive(Debug, Clone)]
pub(super) struct JournalSnapshotCanvas<'a> {
    pub(super) snapshot: &'a JournalTradeSnapshot,
}

impl canvas::Program<Message> for JournalSnapshotCanvas<'_> {
    type State = JournalSnapshotCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        update_snapshot_interaction(state, self.snapshot, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        draw_snapshot_chart(&mut frame, theme, bounds.size(), self.snapshot, state);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.is_dragging() {
            return mouse::Interaction::Grabbing;
        }

        if cursor.position_in(bounds).is_some() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
