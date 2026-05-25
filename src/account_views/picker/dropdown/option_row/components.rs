use crate::message::Message;

use iced::widget::{button, text};
use iced::{Color, Element, Length, Theme};

// ---------------------------------------------------------------------------
// Account Option Row Components
// ---------------------------------------------------------------------------

pub(super) const RENAME_ICON: &str = "✎";

pub(super) fn account_option_row_padding() -> iced::Padding {
    iced::Padding {
        top: 9.0,
        right: 10.0,
        bottom: 9.0,
        left: 14.0,
    }
}

pub(super) fn account_action_button(
    label: &'static str,
    message: Message,
    color: Color,
    active: bool,
) -> Element<'static, Message> {
    button(text(label).size(10).center())
        .on_press(message)
        .padding([7, 8])
        .width(account_action_button_width(label))
        .style(move |theme: &Theme, status| {
            account_action_button_style(theme, status, color, active)
        })
        .into()
}

fn account_action_button_width(label: &str) -> Length {
    if label == RENAME_ICON {
        Length::Fixed(34.0)
    } else {
        Length::Fixed(64.0)
    }
}

fn account_action_button_style(
    theme: &Theme,
    status: button::Status,
    color: Color,
    active: bool,
) -> button::Style {
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
}
