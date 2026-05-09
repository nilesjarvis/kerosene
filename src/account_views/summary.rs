mod connected;
mod controls;
mod disconnected;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;

impl TradingTerminal {
    pub(crate) fn view_account_summary(&self) -> Element<'_, Message> {
        if self.connected_address.is_none() {
            self.view_disconnected_account_summary()
        } else {
            self.view_connected_account_summary()
        }
    }
}
