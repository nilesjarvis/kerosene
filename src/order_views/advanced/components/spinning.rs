use crate::message::Message;

use iced::widget::{canvas, container};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Spinning Gear
// ---------------------------------------------------------------------------

pub(in crate::order_views::advanced) fn spinning_gear(
    phase: f32,
    size: u32,
    color: Color,
) -> Element<'static, Message> {
    container(
        iced::widget::canvas(SpinningGear { phase, color })
            .width(size as f32)
            .height(size as f32),
    )
    .width(size)
    .height(size)
    .center(Fill)
    .into()
}

struct SpinningGear {
    phase: f32,
    color: Color,
}

impl canvas::Program<Message> for SpinningGear {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 1.2;
        if radius <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let stroke = canvas::Stroke::default()
            .with_color(self.color)
            .with_width(1.3)
            .with_line_cap(canvas::LineCap::Round);
        let muted_stroke = canvas::Stroke::default()
            .with_color(Color {
                a: 0.45,
                ..self.color
            })
            .with_width(1.0)
            .with_line_cap(canvas::LineCap::Round);

        frame.stroke(&canvas::Path::circle(center, radius * 0.58), stroke);
        frame.stroke(&canvas::Path::circle(center, radius * 0.22), muted_stroke);

        for i in 0..8 {
            let angle = self.phase + i as f32 * std::f32::consts::TAU / 8.0;
            let inner = radial_point(center, radius * 0.72, angle);
            let outer = radial_point(center, radius, angle);
            let tooth = canvas::Path::new(|path| {
                path.move_to(inner);
                path.line_to(outer);
            });
            frame.stroke(&tooth, stroke);
        }

        for i in 0..4 {
            let angle = self.phase + i as f32 * std::f32::consts::TAU / 4.0;
            let inner = radial_point(center, radius * 0.28, angle);
            let outer = radial_point(center, radius * 0.52, angle);
            let spoke = canvas::Path::new(|path| {
                path.move_to(inner);
                path.line_to(outer);
            });
            frame.stroke(&spoke, muted_stroke);
        }

        vec![frame.into_geometry()]
    }
}

fn radial_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}
