use crate::message::Message;

use iced::widget::{container, text};
use iced::{Color, Element, Theme};

pub(super) fn tooltip_body(body: &'static str) -> Element<'static, Message> {
    container(text(body).size(10))
        .padding([5, 8])
        .max_width(280.0)
        .style(tooltip_container_style)
        .into()
}

fn tooltip_container_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(
            Color {
                a: 0.98,
                ..theme.extended_palette().background.strong.color
            }
            .into(),
        ),
        text_color: Some(theme.palette().text),
        border: iced::Border {
            width: 1.0,
            color: Color {
                a: 0.45,
                ..theme.palette().primary
            },
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}
