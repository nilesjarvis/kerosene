use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use crate::settings_state::{SettingsTab, ThemeSettingsPage};
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
                    ..crate::window_chrome::settings(self.custom_window_chrome_active)
                };
                let (id, task) = window::open(settings);
                self.settings_window_id = Some(id);

                return task.map(Message::WindowOpened);
            }
            Message::OpenIntegrationsSettings => {
                self.settings_active_tab = SettingsTab::Integrations;
                self.settings_theme_page = ThemeSettingsPage::Overview;
                return self.update(Message::OpenSettingsWindow);
            }
            Message::SettingsTabSelected(tab) => {
                self.settings_active_tab = tab;
                self.settings_theme_page = ThemeSettingsPage::Overview;
            }
            Message::ThemeSettingsPageSelected(page) => {
                self.settings_active_tab = SettingsTab::Themes;
                self.settings_theme_page = page;
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
                self.settings_theme_page = ThemeSettingsPage::Overview;
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
                self.encrypted_secret_password = value.into_zeroizing().into();
            }
            Message::EncryptedSecretConfirmChanged(value) => {
                self.encrypted_secret_confirm.zeroize();
                self.encrypted_secret_confirm = value.into_zeroizing().into();
            }
            Message::UnlockEncryptedSecrets => {
                return self.unlock_encrypted_credentials();
            }
            Message::ApplySecretStorageSelection => {
                self.apply_secret_storage_selection();
            }
            Message::ClearConfigs => {
                return self.request_config_clear();
            }
            Message::ConfigsCleared(result) => return self.handle_config_clear_result(result),
            _ => {}
        }

        Task::none()
    }
}
