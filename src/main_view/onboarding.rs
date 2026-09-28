mod animation;

use crate::app_state::TradingTerminal;
use crate::helpers::text_color_for_bg;
use crate::message::Message;
use animation::{
    ONBOARDING_PHASE_PERIOD, ONBOARDING_PHASE_STEP, OnboardingBackdrop, OnboardingGraphic,
};
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, stack, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// App Onboarding
// ---------------------------------------------------------------------------

const ONBOARDING_CONTENT_WIDTH: f32 = 720.0;
const ONBOARDING_GRAPHIC_WIDTH: f32 = 260.0;
const ONBOARDING_GRAPHIC_HEIGHT: f32 = 176.0;

impl TradingTerminal {
    /// Advance the first-run onboarding animation. Unlike `spinner_phase` (an
    /// angle wrapped at TAU), this accumulates and wraps at `ONBOARDING_PHASE_PERIOD`
    /// so the looping welcome visuals never jump when the phase resets.
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

        let content = column![
            iced::widget::canvas(OnboardingGraphic {
                phase: self.onboarding_phase,
            })
            .width(Length::Fixed(ONBOARDING_GRAPHIC_WIDTH))
            .height(Length::Fixed(ONBOARDING_GRAPHIC_HEIGHT)),
            text("Kerosene").size(44).center(),
            text("A GPU-accelerated desktop trading terminal for Hyperliquid.")
                .size(15)
                .center(),
            row![
                market_chip("Live markets", |theme| theme.palette().primary),
                market_chip("Charting", |theme| theme.palette().success),
                market_chip("Automation", |theme| theme.palette().danger),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            button(text("Enter Terminal").size(14).center())
                .on_press(Message::EnterApplication)
                .padding([11, 24])
                .style(onboarding_button_style)
        ]
        .spacing(18)
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

fn market_chip<'a>(label: &'static str, color: fn(&Theme) -> Color) -> Element<'a, Message> {
    container(
        row![chip_dot(color), text(label).size(11)]
            .spacing(7)
            .align_y(Alignment::Center),
    )
    .padding([5, 9])
    .style(move |theme: &Theme| {
        let accent = color(theme);
        container_style::Style {
            background: Some(Color { a: 0.13, ..accent }.into()),
            text_color: Some(Color {
                a: 0.92,
                ..theme.palette().text
            }),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.26, ..accent },
            },
            ..Default::default()
        }
    })
    .into()
}

fn chip_dot<'a>(color: fn(&Theme) -> Color) -> Element<'a, Message> {
    container(Space::new().width(7).height(7))
        .style(move |theme: &Theme| {
            let accent = color(theme);
            container_style::Style {
                background: Some(Color { a: 0.8, ..accent }.into()),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn onboarding_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let base = theme.palette().primary;
    let bg = match status {
        button::Status::Hovered => mix_color(base, theme.palette().success, 0.18),
        button::Status::Pressed => mix_color(base, theme.palette().text, 0.10),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: text_color_for_bg(bg),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color {
                a: 0.42,
                ..theme.palette().text
            },
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
