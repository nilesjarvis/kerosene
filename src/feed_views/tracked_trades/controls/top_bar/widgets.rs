use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, text};
use iced::{Color, Element, Theme};

pub(super) fn tracked_trade_status_dot(color: Color) -> Element<'static, Message> {
    container(Space::new().width(8.0).height(8.0))
        .style(move |_| container_style::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn tracked_trade_reconnect_button() -> Element<'static, Message> {
    button(text("Reconnect").size(10))
        .on_press(Message::ReconnectTrackedTrades)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn tracked_trade_clear_button() -> Element<'static, Message> {
    button(text("Clear").size(10).center())
        .on_press(Message::ClearTrackedTrades)
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
        .into()
}

pub(super) fn tracked_trade_toggle_button(
    label: &'static str,
    enabled: bool,
    primary_when_enabled: bool,
    message: Message,
) -> Element<'static, Message> {
    button(text(label).size(10))
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
