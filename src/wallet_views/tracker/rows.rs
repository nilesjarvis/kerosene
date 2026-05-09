use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Column, text};
use iced::{Element, Theme};

mod row;

// ---------------------------------------------------------------------------
// Wallet Tracker Rows
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_wallet_tracker_list<'a>(&'a self, theme: &Theme) -> Element<'a, Message> {
        let mut list = Column::new().spacing(4);
        if self.wallet_tracker.tracked_addresses.is_empty() {
            return list
                .push(
                    text("No wallets tracked yet. Add an address above.")
                        .size(12)
                        .color(theme.palette().text),
                )
                .into();
        }

        for address in &self.wallet_tracker.tracked_addresses {
            list = list.push(self.view_wallet_tracker_row(address, theme));
        }

        list.into()
    }
}
