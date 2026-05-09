use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, row, text, text_input};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn unlock_credentials_status(&self, theme: &Theme) -> Option<Element<'_, Message>> {
        self.secret_store_status
            .as_ref()
            .map(|(message, is_error)| {
                text(message)
                    .size(11)
                    .color(if *is_error {
                        theme.palette().danger
                    } else {
                        theme.extended_palette().background.weak.text
                    })
                    .width(Fill)
                    .into()
            })
    }

    pub(super) fn unlock_credentials_password_row(&self) -> Element<'_, Message> {
        row![
            text_input("Encryption password", &self.encrypted_secret_password)
                .style(helpers::text_input_style)
                .on_input(Message::EncryptedSecretPasswordChanged)
                .on_submit(Message::UnlockEncryptedSecrets)
                .secure(true)
                .size(12)
                .padding(8)
                .width(Fill),
            button(text("Unlock").size(12).center())
                .on_press(Message::UnlockEncryptedSecrets)
                .padding([8, 14])
                .style(|theme: &Theme, _status| button::Style {
                    background: Some(theme.palette().primary.into()),
                    text_color: theme.palette().background,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

pub(super) fn unlock_credentials_action_row() -> Element<'static, Message> {
    row![
        unlock_action_button(
            "Use Without Credentials",
            Message::DismissUnlockCredentialsPopup,
            false,
        ),
        unlock_action_button(
            "Open Storage Settings",
            Message::OpenCredentialStorageSettings,
            true,
        ),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn unlock_action_button(
    label: &'static str,
    message: Message,
    primary_text: bool,
) -> Element<'static, Message> {
    button(text(label).size(11))
        .on_press(message)
        .padding([6, 10])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if primary_text {
                    theme.palette().primary
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
