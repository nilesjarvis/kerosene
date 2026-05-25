use crate::message::Message;

use iced::widget::{button, text};
use iced::{Color, Element, Theme};

// ---------------------------------------------------------------------------
// Account View Styling
// ---------------------------------------------------------------------------

pub(in crate::account_views) fn compact_action_button(
    label: &'static str,
    color: Color,
    message: Message,
) -> Element<'static, Message> {
    button(text(label).size(10).center().color(color))
        .on_press(message)
        .padding([1, 6])
        .style(move |_theme: &Theme, _status| button::Style {
            background: Some(Color { a: 0.15, ..color }.into()),
            text_color: color,
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
