use super::mix_color;
use crate::message::Message;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, mouse};

// One gentle cycle every ~31 seconds on the shared 40 ms UI timer.
pub(super) const ONBOARDING_PHASE_PERIOD: f32 = std::f32::consts::TAU;
pub(super) const ONBOARDING_PHASE_STEP: f32 = 0.008;

pub(super) struct OnboardingBackdrop {
    pub(super) phase: f32,
}

impl canvas::Program<Message> for OnboardingBackdrop {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let base = theme.extended_palette().background.base.color;
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            onboarding_gradient(bounds.size(), self.phase, base, theme.palette().primary),
        );

        // Fade the wash into the background at the edges, keeping the content quiet.
        let fade = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(0.0, bounds.height))
            .add_stop(0.0, Color { a: 0.72, ..base })
            .add_stop(0.45, Color { a: 0.0, ..base })
            .add_stop(1.0, Color { a: 0.9, ..base });
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), fade);

        vec![frame.into_geometry()]
    }
}

pub(super) fn onboarding_gradient(
    size: Size,
    phase: f32,
    base: Color,
    accent: Color,
) -> canvas::gradient::Linear {
    let drift = phase.sin() * 0.12;
    let glow = 0.12 + phase.cos() * 0.025;
    canvas::gradient::Linear::new(
        Point::new(size.width * (-0.15 + drift), size.height * 0.1),
        Point::new(size.width * (1.05 + drift), size.height * (0.9 + drift)),
    )
    .add_stop(0.0, base)
    .add_stop(0.25, mix_color(base, accent, glow * 0.35))
    .add_stop(0.5, mix_color(base, accent, glow))
    .add_stop(0.75, mix_color(base, accent, glow * 0.35))
    .add_stop(1.0, base)
}
