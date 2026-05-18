use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use crate::settings_state::SettingsTab;
use iced::{Size, Task, window};
use zeroize::Zeroize;

impl TradingTerminal {
    pub(crate) fn update_settings(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenSettingsWindow => {
                self.add_widget_menu_open = false;
                self.layout_menu_open = false;
                self.layout_rename_index = None;
                self.layout_rename_input.clear();
                self.account_picker_open = false;
                self.account_picker_rename_index = None;
                if let Some(id) = self.settings_window_id {
                    return window::gain_focus(id);
                }

                let settings = window::Settings {
                    size: Size::new(800.0, 600.0),
                    ..crate::window_chrome::settings()
                };
                let (id, task) = window::open(settings);
                self.settings_window_id = Some(id);

                return task.map(Message::WindowOpened);
            }
            Message::SettingsTabSelected(tab) => {
                self.settings_active_tab = tab;
            }
            Message::OpenUnlockCredentialsPopup => {
                self.show_unlock_credentials_popup = self.encrypted_credentials_locked();
            }
            Message::DismissUnlockCredentialsPopup => {
                self.show_unlock_credentials_popup = false;
                self.encrypted_secret_password.zeroize();
            }
            Message::OpenCredentialStorageSettings => {
                self.show_unlock_credentials_popup = false;
                self.settings_active_tab = SettingsTab::Storage;
                return self.update(Message::OpenSettingsWindow);
            }
            Message::SecretStorageSelectionChanged(mode) => {
                self.secret_storage_selection = mode;
                if mode == config::CredentialStorageMode::OsKeychain {
                    self.encrypted_secret_password.zeroize();
                    self.encrypted_secret_confirm.zeroize();
                }
            }
            Message::EncryptedSecretPasswordChanged(value) => {
                self.encrypted_secret_password.zeroize();
                self.encrypted_secret_password = value.into();
            }
            Message::EncryptedSecretConfirmChanged(value) => {
                self.encrypted_secret_confirm.zeroize();
                self.encrypted_secret_confirm = value.into();
            }
            Message::UnlockEncryptedSecrets => {
                self.unlock_encrypted_credentials();
            }
            Message::ApplySecretStorageSelection => {
                self.apply_secret_storage_selection();
            }
            Message::ClearConfigs => {
                let profiles = self.persisted_accounts_snapshot();
                return Task::perform(
                    async move { config::clear_all_configs(&profiles) },
                    Message::ConfigsCleared,
                );
            }
            Message::ConfigsCleared(result) => match result {
                Ok(summary) => self.apply_config_clear_to_runtime(summary),
                Err(e) => {
                    let message = format!("Config clear failed: {e}");
                    self.secret_store_status = Some((message.clone(), true));
                    self.push_toast(message, true);
                }
            },
            _ => {}
        }

        Task::none()
    }
}
