use crate::message::Message;

use iced::Theme;
use iced::widget::{button, text};

// ---------------------------------------------------------------------------
// Size Input Components
// ---------------------------------------------------------------------------

pub(super) fn denomination_button<'a>(label: &'static str) -> button::Button<'a, Message> {
    button(text(label).size(10).center())
        .on_press(Message::ToggleOrderDenomination)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
}
