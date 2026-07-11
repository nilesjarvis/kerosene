use super::scale::{FlowChartScale, cumulative_line_points};
use crate::message::Message;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Cumulative Inflow Line
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct CumulativeInflowLine {
    pub(super) values: Vec<f64>,
    pub(super) scale: FlowChartScale,
}

impl canvas::Program<Message> for CumulativeInflowLine {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        let points = cumulative_line_points(&self.values, bounds.width, bounds.height, self.scale);
        if points.is_empty() {
            return vec![frame.into_geometry()];
        }

        let line_color = theme.palette().primary;
        if points.len() == 1 {
            frame.fill(&canvas::Path::circle(points[0], 2.8), line_color);
            return vec![frame.into_geometry()];
        }

        let line = canvas::Path::new(|path| {
            for (idx, point) in points.iter().copied().enumerate() {
                if idx == 0 {
                    path.move_to(point);
                } else {
                    path.line_to(point);
                }
            }
        });
        frame.stroke(
            &line,
            canvas::Stroke::default()
                .with_color(line_color)
                .with_width(2.0)
                .with_line_cap(canvas::LineCap::Round)
                .with_line_join(canvas::LineJoin::Round),
        );

        for point in points {
            frame.fill(&canvas::Path::circle(point, 2.2), line_color);
            frame.stroke(
                &canvas::Path::circle(point, 2.2),
                canvas::Stroke::default()
                    .with_color(theme.extended_palette().background.weak.color)
                    .with_width(1.0),
            );
        }

        vec![frame.into_geometry()]
    }
}
