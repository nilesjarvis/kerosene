use crate::message::Message;

use iced::widget::{button, text};
use iced::{Color, Element, Length, Theme};

// ---------------------------------------------------------------------------
// Layout Switcher Actions
// ---------------------------------------------------------------------------

pub(super) fn layout_header_update_button(enabled: bool) -> Element<'static, Message> {
    let button = button(text("Update").size(10).center())
        .padding([4, 8])
        .style(layout_action_style);

    if enabled {
        button.on_press(Message::UpdateActiveLayout).into()
    } else {
        button.into()
    }
}

pub(super) fn layout_action_button(
    label: &'static str,
    message: Message,
    color: Color,
    active: bool,
    width: f32,
) -> Element<'static, Message> {
    button(text(label).size(10).center())
        .on_press(message)
        .padding([6, 6])
        .width(Length::Fixed(width))
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: color,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active { color } else { Color::TRANSPARENT },
                },
                ..Default::default()
            }
        })
        .into()
}

fn layout_action_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}
