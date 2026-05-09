mod configuration;
mod credentials;

use self::configuration::configuration_actions;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Element;
use iced::widget::{Column, rule, text};

impl TradingTerminal {
    pub(crate) fn view_settings_storage_section(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mut storage_section = Column::new()
            .spacing(12)
            .push(text("Storage").size(16).color(current_theme.palette().text))
            .push(rule::horizontal(1))
            .push(self.view_credential_storage_controls())
            .push(rule::horizontal(1))
            .push(configuration_actions(&current_theme));

        if let Some((status, is_error)) = &self.secret_store_status {
            storage_section = storage_section.push(text(status).size(11).color(if *is_error {
                current_theme.palette().danger
            } else {
                current_theme.extended_palette().background.weak.text
            }));
        }

        storage_section.into()
    }
}
