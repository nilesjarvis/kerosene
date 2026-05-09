use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, text};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_journal_title(&self) -> Element<'static, Message> {
        let theme = self.theme();
        let title = text("Trading Journal")
            .size(20)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.palette().text),
            });
        let active_profile = self.accounts.get(self.active_account_index);
        let account_label = active_profile
            .map(|profile| profile.name.as_str())
            .unwrap_or("No account");
        let active_profile_address = active_profile
            .map(|profile| profile.wallet_address.trim())
            .filter(|address| !address.is_empty());
        let address_label = self
            .journal
            .loaded_address
            .as_deref()
            .or(active_profile_address)
            .or(self.connected_address.as_deref())
            .map(|address| {
                let display = self.wallet_display(address);
                if display.has_label {
                    format!("{} ({})", display.primary, display.secondary)
                } else {
                    display.primary
                }
            })
            .unwrap_or_else(|| "Disconnected".to_string());
        let mode_label = if self.active_account_is_ghost() {
            "Ghost/session only"
        } else if self.active_account_can_trade() {
            "Trading"
        } else {
            "Watch only"
        };

        Column::new()
            .spacing(2)
            .push(title)
            .push(
                text(format!(
                    "{} | {} | {}",
                    account_label, address_label, mode_label
                ))
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            )
            .into()
    }
}
