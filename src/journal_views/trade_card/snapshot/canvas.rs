use super::drawing::draw_snapshot_chart;
use super::interaction::{JournalSnapshotCanvasState, update_snapshot_interaction};
use super::plot::point_in_snapshot_plot;
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
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        let hovered = draw_snapshot_chart(
            &mut frame,
            theme,
            self.snapshot,
            state,
            cursor.position_in(bounds),
        );

        let mut layers = vec![frame.into_geometry()];
        if let Some(group) = hovered {
            let mut tooltip = canvas::Frame::new(renderer, bounds.size());
            group.draw_tooltip(&mut tooltip, theme, &self.snapshot.coin);
            layers.push(tooltip.into_geometry());
        }
        layers
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

        if cursor
            .position_in(bounds)
            .is_some_and(|pos| point_in_snapshot_plot(bounds.size(), pos))
        {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}
