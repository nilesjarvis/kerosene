mod connected;
mod controls;
mod disconnected;
mod layout_switcher;
mod menus;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;

impl TradingTerminal {
    pub(crate) fn view_account_summary(&self) -> Element<'_, Message> {
        let content = if self.connected_address.is_none() {
            self.view_disconnected_account_summary()
        } else {
            self.view_connected_account_summary()
        };

        self.view_account_summary_with_menus(content)
    }
}
