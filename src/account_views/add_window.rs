use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::{self, text_color_for_bg};
use crate::message::Message;
use crate::signing;

use iced::widget::container as container_style;
use iced::widget::{Space, button, checkbox, column, container, row, rule, text, text_input};
use iced::{Alignment, Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Add Account Window
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeedbackTone {
    Hint,
    Valid,
    Warning,
    Error,
}

fn address_feedback(address_input: &str, existing_addresses: &[String]) -> (String, FeedbackTone) {
    if address_input.trim().is_empty() {
        return (
            "The master account whose positions and orders this profile follows.".to_string(),
            FeedbackTone::Hint,
        );
    }
    let Some(address) = TradingTerminal::normalize_wallet_address(address_input) else {
        return (
            "Not a valid wallet address (expected 0x followed by 40 hex characters).".to_string(),
            FeedbackTone::Error,
        );
    };
    if existing_addresses.iter().any(|existing| {
        TradingTerminal::normalize_wallet_address(existing).as_deref() == Some(address.as_str())
    }) {
        return (
            "A saved profile already uses this address; adding another is allowed.".to_string(),
            FeedbackTone::Warning,
        );
    }
    (
        format!("Valid address {}", TradingTerminal::short_address(&address)),
        FeedbackTone::Valid,
    )
}

fn key_feedback(key_input: &str) -> (String, FeedbackTone) {
    let key = key_input.trim();
    if key.is_empty() {
        return (
            "Leave empty for a watch-only profile; you can save a key later.".to_string(),
            FeedbackTone::Hint,
        );
    }
    match signing::agent_wallet_address_for_key(key) {
        Ok(agent_address) => (
            format!(
                "Valid key · agent wallet {}",
                TradingTerminal::short_address(&agent_address)
            ),
            FeedbackTone::Valid,
        ),
        Err(error) => (error, FeedbackTone::Error),
    }
}

struct StorageNotice {
    message: String,
    blocks_key_save: bool,
}

fn storage_notice(
    mode: config::CredentialStorageMode,
    locked: bool,
    password_ready: bool,
) -> StorageNotice {
    match mode {
        config::CredentialStorageMode::OsKeychain => StorageNotice {
            message: "The trading key is stored in your OS keychain.".to_string(),
            blocks_key_save: false,
        },
        config::CredentialStorageMode::EncryptedConfig if locked => StorageNotice {
            message:
                "Encrypted credentials are locked; unlock them in Settings > Storage to save a trading key."
                    .to_string(),
            blocks_key_save: true,
        },
        config::CredentialStorageMode::EncryptedConfig if !password_ready => StorageNotice {
            message:
                "Encrypted credential storage has no password yet; set one in Settings > Storage to save a trading key."
                    .to_string(),
            blocks_key_save: true,
        },
        config::CredentialStorageMode::EncryptedConfig => StorageNotice {
            message: "The trading key is stored in the password-encrypted config.".to_string(),
            blocks_key_save: false,
        },
    }
}

impl TradingTerminal {
    pub(crate) fn view_add_account_window(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(state) = &self.add_account_window else {
            return Space::new().into();
        };

        let existing_addresses: Vec<String> = self
            .accounts
            .iter()
            .map(|profile| profile.wallet_address.clone())
            .collect();
        let (address_message, address_tone) =
            address_feedback(&state.address_input, &existing_addresses);
        let (key_message, key_tone) = key_feedback(&state.key_input);
        let notice = storage_notice(
            self.secret_storage_mode,
            self.encrypted_credentials_locked(),
            !self.encrypted_secret_password.trim().is_empty()
                || self.secret_storage_mode == config::CredentialStorageMode::OsKeychain,
        );

        let has_key = !state.key_input.trim().is_empty();
        let submit_enabled = address_tone != FeedbackTone::Error
            && !state.address_input.trim().is_empty()
            && key_tone != FeedbackTone::Error
            && !(has_key && notice.blocks_key_save);

        let default_name = format!("Account {}", self.persisted_accounts_snapshot().len() + 1);
        let name_input = text_input(&default_name, &state.name_input)
            .style(helpers::text_input_style)
            .on_input(|value| Message::AddAccountNameChanged(value.into()))
            .size(12)
            .padding([6, 8])
            .width(Fill);

        let address_input = text_input("0x…", &state.address_input)
            .style(helpers::text_input_style)
            .on_input(|value| Message::AddAccountAddressChanged(value.into()))
            .size(12)
            .padding([6, 8])
            .width(Fill);

        let key_input = text_input("Agent private key (enables trading)", &state.key_input)
            .style(helpers::text_input_style)
            .on_input(|value| Message::AddAccountKeyChanged(value.into()))
            .secure(true)
            .size(12)
            .padding([6, 8])
            .width(Fill);

        let mut storage_row = row![
            text(notice.message.clone())
                .size(11)
                .color(if notice.blocks_key_save && has_key {
                    theme.palette().danger
                } else if notice.blocks_key_save {
                    theme.palette().warning
                } else {
                    theme.extended_palette().background.weak.text
                })
                .width(Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if notice.blocks_key_save {
            storage_row = storage_row.push(
                button(text("Open Storage Settings").size(10).center())
                    .on_press(Message::OpenCredentialStorageSettings)
                    .padding([4, 10])
                    .style(secondary_button_style),
            );
        }

        let switch_toggle = checkbox(state.switch_on_add)
            .label("Switch to this account after adding")
            .on_toggle(Message::AddAccountSwitchToggled)
            .size(14)
            .spacing(8)
            .text_size(12);

        let cancel_button = button(text("Cancel").size(12).center())
            .on_press(Message::AddAccountCancel)
            .padding([7, 16])
            .style(secondary_button_style);
        let submit_button = button(text("Add Account").size(12).center())
            .on_press_maybe(submit_enabled.then_some(Message::AddAccountSubmit))
            .padding([7, 16])
            .style(primary_button_style);

        let mut content = column![
            text("Add Hyperliquid Account")
                .size(16)
                .color(theme.palette().text),
            text("Nothing is saved until you press Add Account.")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            rule::horizontal(1),
            self.view_add_account_field("Profile name (optional)", name_input.into()),
            self.view_add_account_field("Master account address", address_input.into()),
            feedback_line(address_message, address_tone, &theme),
            rule::horizontal(1),
            self.view_add_account_field("Agent private key (optional)", key_input.into()),
            feedback_line(key_message, key_tone, &theme),
            storage_row,
            rule::horizontal(1),
            switch_toggle,
        ]
        .spacing(10)
        .width(Fill);

        if let Some(error) = &state.error {
            content = content.push(
                text(error.clone())
                    .size(11)
                    .color(theme.palette().danger)
                    .width(Fill),
            );
        }

        content = content.push(
            row![Space::new().width(Fill), cancel_button, submit_button,]
                .spacing(10)
                .align_y(Alignment::Center),
        );

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(18)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }

    fn view_add_account_field<'a>(
        &'a self,
        label: &'static str,
        input: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        column![
            text(label)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            input,
        ]
        .spacing(4)
        .width(Fill)
        .into()
    }
}

fn feedback_line(
    message: String,
    tone: FeedbackTone,
    theme: &Theme,
) -> iced::widget::Column<'static, Message> {
    let color = match tone {
        FeedbackTone::Hint => theme.extended_palette().background.weak.text,
        FeedbackTone::Valid => theme.palette().success,
        FeedbackTone::Warning => theme.palette().warning,
        FeedbackTone::Error => theme.palette().danger,
    };
    column![text(message).size(11).color(color).width(Fill)]
}

fn primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let base = theme.palette().primary;
    let bg = match status {
        button::Status::Hovered => Color { a: 0.85, ..base },
        button::Status::Pressed => Color { a: 0.7, ..base },
        button::Status::Disabled => Color { a: 0.3, ..base },
        button::Status::Active => base,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: text_color_for_bg(bg),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn secondary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS_A: &str = "0xabc0000000000000000000000000000000000000";
    const VALID_KEY: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";

    #[test]
    fn address_feedback_flags_empty_invalid_duplicate_and_valid_inputs() {
        let existing = vec![ADDRESS_A.to_string()];

        let (_, tone) = address_feedback("", &existing);
        assert_eq!(tone, FeedbackTone::Hint);

        let (message, tone) = address_feedback("0x1234", &existing);
        assert_eq!(tone, FeedbackTone::Error);
        assert!(message.contains("Not a valid wallet address"));

        let (message, tone) =
            address_feedback(&ADDRESS_A.to_uppercase().replace("0X", "0x"), &existing);
        assert_eq!(tone, FeedbackTone::Warning);
        assert!(message.contains("already uses this address"));

        let (message, tone) =
            address_feedback("0xdef0000000000000000000000000000000000000", &existing);
        assert_eq!(tone, FeedbackTone::Valid);
        assert!(message.contains("Valid address"));
    }

    #[test]
    fn key_feedback_derives_agent_wallet_for_valid_keys() {
        let (_, tone) = key_feedback("");
        assert_eq!(tone, FeedbackTone::Hint);

        let (message, tone) = key_feedback("nope");
        assert_eq!(tone, FeedbackTone::Error);
        assert!(message.contains("Invalid private key hex"));

        let (message, tone) = key_feedback(VALID_KEY);
        assert_eq!(tone, FeedbackTone::Valid);
        assert!(message.contains("agent wallet 0x7e5f...5bdf"));
    }

    #[test]
    fn storage_notice_blocks_key_saves_only_when_encrypted_storage_is_unusable() {
        let keychain = storage_notice(config::CredentialStorageMode::OsKeychain, false, true);
        assert!(!keychain.blocks_key_save);

        let unlocked = storage_notice(config::CredentialStorageMode::EncryptedConfig, false, true);
        assert!(!unlocked.blocks_key_save);

        let locked = storage_notice(config::CredentialStorageMode::EncryptedConfig, true, false);
        assert!(locked.blocks_key_save);
        assert!(locked.message.contains("locked"));

        let no_password =
            storage_notice(config::CredentialStorageMode::EncryptedConfig, false, false);
        assert!(no_password.blocks_key_save);
        assert!(no_password.message.contains("no password"));
    }
}
