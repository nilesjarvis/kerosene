use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers;
use crate::message::Message;

use iced::Fill;
use iced::widget::{Column, button, pick_list, row, text, text_input};

impl TradingTerminal {
    pub(super) fn view_credential_storage_controls(&self) -> Column<'_, Message> {
        let current_theme = self.theme();
        let storage_mode_options = vec![
            config::CredentialStorageMode::OsKeychain,
            config::CredentialStorageMode::EncryptedConfig,
        ];
        let encrypted_selected =
            self.secret_storage_selection == config::CredentialStorageMode::EncryptedConfig;
        let encrypted_locked = self.secret_storage_mode
            == config::CredentialStorageMode::EncryptedConfig
            && self.encrypted_secrets.is_some()
            && !self.encrypted_secrets_unlocked;
        let mut storage_selector_row = row![
            text("Credential Storage")
                .size(14)
                .color(current_theme.palette().text)
                .width(Fill),
            pick_list(
                storage_mode_options,
                Some(self.secret_storage_selection),
                Message::SecretStorageSelectionChanged,
            )
            .width(iced::Length::Fixed(190.0)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        if !encrypted_selected {
            storage_selector_row = storage_selector_row.push(
                button(text("Use OS Keychain").size(12))
                    .padding([6, 12])
                    .on_press_maybe(
                        (self.secret_storage_selection != self.secret_storage_mode)
                            .then_some(Message::ApplySecretStorageSelection),
                    ),
            );
        }

        let mut credential_section = Column::new().spacing(8).push(storage_selector_row);
        if encrypted_selected {
            credential_section =
                credential_section.push(encrypted_password_row(self, encrypted_locked));
            if !encrypted_locked {
                credential_section = credential_section.push(encrypted_confirm_input(self));
            }
            credential_section = credential_section.push(encrypted_state_label(self));
        }

        credential_section
    }
}

fn encrypted_password_row<'a>(
    terminal: &'a TradingTerminal,
    encrypted_locked: bool,
) -> iced::widget::Row<'a, Message> {
    row![
        text_input("Password", &terminal.encrypted_secret_password)
            .style(helpers::text_input_style)
            .on_input(|value| Message::EncryptedSecretPasswordChanged(value.into()))
            .on_submit(if encrypted_locked {
                Message::UnlockEncryptedSecrets
            } else {
                Message::ApplySecretStorageSelection
            })
            .secure(true)
            .size(12)
            .padding(6)
            .width(Fill),
        encrypted_action_button(terminal, encrypted_locked),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
}

fn encrypted_confirm_input(terminal: &TradingTerminal) -> iced::widget::TextInput<'_, Message> {
    text_input("Confirm password", &terminal.encrypted_secret_confirm)
        .style(helpers::text_input_style)
        .on_input(|value| Message::EncryptedSecretConfirmChanged(value.into()))
        .on_submit(Message::ApplySecretStorageSelection)
        .secure(true)
        .size(12)
        .padding(6)
        .width(Fill)
}

fn encrypted_action_button(
    terminal: &TradingTerminal,
    encrypted_locked: bool,
) -> iced::widget::Button<'static, Message> {
    if encrypted_locked {
        button(text("Unlock").size(12))
            .padding([6, 12])
            .on_press(Message::UnlockEncryptedSecrets)
    } else if terminal.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig {
        button(text("Update").size(12))
            .padding([6, 12])
            .on_press(Message::ApplySecretStorageSelection)
    } else {
        button(text("Encrypt").size(12))
            .padding([6, 12])
            .on_press(Message::ApplySecretStorageSelection)
    }
}

fn encrypted_state_label(terminal: &TradingTerminal) -> iced::widget::Text<'_> {
    let current_theme = terminal.theme();
    let encrypted_state = if terminal.encrypted_secrets_unlocked {
        "Encrypted credentials unlocked"
    } else if terminal.encrypted_secrets.is_some() {
        "Encrypted credentials locked"
    } else {
        "No encrypted credentials saved"
    };

    text(encrypted_state)
        .size(11)
        .color(current_theme.extended_palette().background.weak.text)
}
