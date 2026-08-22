use crate::message::Message;

use iced::widget::rule;
use iced::{Color, Element, Theme};

pub fn section_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.12,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}
