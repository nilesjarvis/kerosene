use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{container, text};
use iced::{Color, Element, Theme};

pub(super) fn calendar_badge(
    label: &str,
    bg: Color,
    fg: Color,
    width: f32,
) -> Element<'static, Message> {
    container(text(label.to_string()).size(10).center().color(fg))
        .padding([1, 5])
        .width(width)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(bg.into()),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn calendar_row_style(row_bg: Color) -> container_style::Style {
    container_style::Style {
        background: Some(row_bg.into()),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
