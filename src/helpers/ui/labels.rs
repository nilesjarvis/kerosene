use crate::message::Message;
use iced::widget::{container, row, rule, text};
use iced::{Border, Color, Element, Theme};

// ---------------------------------------------------------------------------
// Label Widgets
// ---------------------------------------------------------------------------

pub fn label_value(label: impl ToString, value: impl ToString) -> Element<'static, Message> {
    label_value_with_color(label, value, None)
}

pub fn label_value_colored(
    label: impl ToString,
    value: impl ToString,
    value_color: Color,
) -> Element<'static, Message> {
    label_value_with_color(label, value, Some(value_color))
}

fn label_value_with_color(
    label: impl ToString,
    value: impl ToString,
    value_color: Option<Color>,
) -> Element<'static, Message> {
    let value = text(value.to_string())
        .size(13)
        .font(crate::app_fonts::monospace_font());
    let value = if let Some(value_color) = value_color {
        value.color(value_color)
    } else {
        value
    };

    container(
        row![
            text(label.to_string())
                .size(11)
                .style(|t: &Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.text)
                }),
            value,
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .into()
}

pub fn vertical_spacer() -> Element<'static, Message> {
    container(rule::vertical(1)).height(16).into()
}

/// Small "GROWTH" chip marking HIP-3 markets whose deployer has them in
/// growth mode (reduced fees). Green = success palette, matching gain washes.
pub fn growth_mode_chip() -> Element<'static, Message> {
    container(
        text("GROWTH")
            .size(10)
            .font(crate::app_fonts::monospace_font()),
    )
    .padding([1, 5])
    .style(|theme: &Theme| container::Style {
        background: Some(
            Color {
                a: 0.14,
                ..theme.palette().success
            }
            .into(),
        ),
        text_color: Some(theme.palette().success),
        border: Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
