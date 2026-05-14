use crate::account::{
    fetch_wallet_tracker_open_order_count_scoped, fetch_wallet_tracker_snapshot_scoped,
};
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn start_wallet_tracker_core_refresh(&mut self, address: String) -> Task<Message> {
        self.wallet_tracker
            .rows
            .entry(address.clone())
            .or_default()
            .loading = true;
        let scope = self.account_data_fetch_scope();
        Task::perform(
            fetch_wallet_tracker_snapshot_scoped(address.clone(), scope),
            move |r| Message::WalletTrackerLoaded(address.clone(), Box::new(r)),
        )
    }

    pub(crate) fn start_wallet_tracker_order_refresh(&mut self, address: String) -> Task<Message> {
        self.wallet_tracker
            .rows
            .entry(address.clone())
            .or_default()
            .order_loading = true;
        let scope = self.account_data_fetch_scope();
        Task::perform(
            fetch_wallet_tracker_open_order_count_scoped(address.clone(), scope),
            move |r| Message::WalletTrackerOrdersLoaded(address.clone(), Box::new(r)),
        )
    }

    pub(crate) fn refresh_next_wallet_tracker_core(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if let Some(address) = self.wallet_tracker_next_core_address(now_ms) {
            return self.start_wallet_tracker_core_refresh(address);
        }
        Task::none()
    }

    pub(crate) fn refresh_next_wallet_tracker_orders(&mut self) -> Task<Message> {
        let now_ms = Self::now_ms();
        if let Some(address) = self.wallet_tracker_next_order_address(now_ms) {
            return self.start_wallet_tracker_order_refresh(address);
        }
        Task::none()
    }
}
