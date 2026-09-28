use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::text::Wrapping;
use iced::widget::{Text, button, container, text};
use iced::{Color, Element, Theme};

pub(super) fn feed_header_text(label: &'static str, color: Color) -> Text<'static> {
    text(label).size(11).color(color).wrapping(Wrapping::None)
}

pub(super) fn feed_settings_dropdown(
    content: impl Into<Element<'static, Message>>,
) -> Element<'static, Message> {
    container(content)
        .padding([6, 8])
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.32,
                    ..theme.extended_palette().background.strong.color
                },
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn feed_toggle_button(
    label: &'static str,
    enabled: bool,
    primary_when_enabled: bool,
    message: Message,
    text_size: f32,
) -> Element<'static, Message> {
    button(text(label).size(text_size))
        .on_press(message)
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if enabled {
                    if primary_when_enabled {
                        theme.palette().primary
                    } else {
                        theme.palette().success
                    }
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
