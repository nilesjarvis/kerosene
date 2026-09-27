use crate::app_state::TradingTerminal;
use crate::helpers::text_color_for_bg;
use crate::message::Message;
use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, stack, text};
use iced::{Alignment, Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme, mouse};

// ---------------------------------------------------------------------------
// App Onboarding
// ---------------------------------------------------------------------------

const ONBOARDING_CONTENT_WIDTH: f32 = 480.0;
// One gentle cycle every ~31 seconds on the shared 40 ms UI timer.
const ONBOARDING_PHASE_PERIOD: f32 = std::f32::consts::TAU;
const ONBOARDING_PHASE_STEP: f32 = 0.008;

struct OnboardingBackdrop {
    phase: f32,
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

fn onboarding_gradient(
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

impl TradingTerminal {
    /// Keep the slow gradient motion bounded and seamless across each cycle.
    pub(crate) fn advance_onboarding_phase(&mut self) {
        self.onboarding_phase =
            (self.onboarding_phase + ONBOARDING_PHASE_STEP).rem_euclid(ONBOARDING_PHASE_PERIOD);
    }

    pub(super) fn view_onboarding(&self) -> Element<'_, Message> {
        self.view_onboarding_body(None)
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(super) fn view_onboarding_with_top_bar<'a>(
        &'a self,
        top_bar: Element<'a, Message>,
    ) -> Element<'a, Message> {
        self.view_onboarding_body(Some(top_bar))
    }

    fn view_onboarding_body<'a>(
        &'a self,
        top_bar: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let backdrop = iced::widget::canvas(OnboardingBackdrop {
            phase: self.onboarding_phase,
        })
        .width(Fill)
        .height(Fill);

        let theme = self.theme();
        let content = column![
            column![
                text("Kerosene").size(44).center(),
                text("Your Hyperliquid trading terminal.")
                    .size(14)
                    .color(mix_color(
                        theme.palette().text,
                        theme.palette().background,
                        0.35
                    ))
                    .center(),
            ]
            .spacing(12)
            .align_x(Alignment::Center),
            Space::new().height(16),
            button(text("Start").size(14).center())
                .on_press(Message::EnterApplication)
                .width(160)
                .padding([13, 24])
                .style(onboarding_button_style)
        ]
        .spacing(16)
        .width(Fill)
        .align_x(Alignment::Center);

        // Outer container fills the window and centers the width-capped content;
        // the inner container applies the max width. Collapsing both into a single
        // `max_width(..).center(..)` container leaves the content pinned to the left
        // edge on screens wider than ONBOARDING_CONTENT_WIDTH, because the stack
        // anchors each layer at the top-left rather than centering it.
        let content_layer = container(
            container(content)
                .width(Fill)
                .max_width(ONBOARDING_CONTENT_WIDTH)
                .padding([28, 24]),
        )
        .width(Fill)
        .height(Fill)
        .center(Fill);

        let body = container(stack![backdrop, content_layer].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            });

        match top_bar {
            Some(top_bar) => column![top_bar, body].width(Fill).height(Fill).into(),
            None => body.into(),
        }
    }
}

fn onboarding_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let base = theme.palette().primary;
    let bg = match status {
        button::Status::Hovered => mix_color(base, theme.palette().text, 0.12),
        button::Status::Pressed => mix_color(base, theme.palette().background, 0.12),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: text_color_for_bg(bg),
        border: iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[cfg(test)]
mod tests;
