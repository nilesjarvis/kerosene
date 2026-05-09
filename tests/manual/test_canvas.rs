use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

struct SpreadChart<'a> {
    data: &'a std::collections::VecDeque<(std::time::Instant, f64)>,
}

impl<'a> canvas::Program<crate::Message> for SpreadChart<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.data.is_empty() {
            return vec![frame.into_geometry()];
        }

        // ...
        vec![frame.into_geometry()]
    }
}
