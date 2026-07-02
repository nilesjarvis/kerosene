use crate::message::Message;

use iced::widget::{button, container, text};
use iced::{Element, Fill, Theme};

pub(super) fn dropdown_title(theme: &Theme) -> Element<'static, Message> {
    container(text("Accounts").size(12).color(theme.palette().text))
        .padding([8, 10])
        .width(Fill)
        .into()
}

pub(super) fn section_label(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    container(
        text(label)
            .size(10)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill),
    )
    .padding([6, 10])
    .width(Fill)
    .into()
}

pub(super) fn empty_saved_profiles(theme: &Theme) -> Element<'static, Message> {
    container(
        text("No saved profiles")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
    )
    .padding([8, 10])
    .width(Fill)
    .into()
}

pub(super) fn add_account_button() -> Element<'static, Message> {
    button(text("+ Account").size(11).center().width(Fill))
        .on_press(Message::OpenAddAccountWindow)
        .padding([8, 10])
        .width(Fill)
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn schwab_connect_button() -> Element<'static, Message> {
    button(text("+ Schwab").size(11).center().width(Fill))
        .on_press(Message::OpenIntegrationsSettings)
        .padding([8, 10])
        .width(Fill)
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn disconnect_account_button() -> Element<'static, Message> {
    button(text("Disconnect").size(11).center().width(Fill))
        .on_press(Message::DisconnectWallet)
        .padding([8, 10])
        .width(Fill)
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().danger,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
