use crate::message::Message;
use iced::widget::{container, row, rule, text};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Label Widgets
// ---------------------------------------------------------------------------

pub fn label_value(label: impl ToString, value: impl ToString) -> Element<'static, Message> {
    container(
        row![
            text(label.to_string())
                .size(11)
                .style(|t: &Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.text)
                }),
            text(value.to_string()).size(13).font(iced::Font::MONOSPACE),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .into()
}

pub fn label_value_colored(
    label: impl ToString,
    value: impl ToString,
    value_color: Color,
) -> Element<'static, Message> {
    container(
        row![
            text(label.to_string())
                .size(11)
                .style(|t: &Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.text)
                }),
            text(value.to_string())
                .size(13)
                .font(iced::Font::MONOSPACE)
                .color(value_color),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .into()
}

pub fn vertical_spacer() -> Element<'static, Message> {
    container(rule::vertical(1)).height(16).into()
}
