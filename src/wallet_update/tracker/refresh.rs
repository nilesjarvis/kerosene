use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_wallet_tracker_refresh(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WalletTrackerRefresh => {
                self.queue_wallet_tracker_core_refresh_all();
                self.refresh_next_wallet_tracker_core()
            }
            Message::WalletTrackerRefreshDue => self.refresh_next_wallet_tracker_core(),
            Message::WalletTrackerRefreshOne(address) => {
                if self.wallet_tracker.tracked_addresses.contains(&address) {
                    self.queue_wallet_tracker_core_refresh(address);
                    return self.refresh_next_wallet_tracker_core();
                }
                Task::none()
            }
            Message::WalletTrackerRefreshOrdersDue => self.refresh_next_wallet_tracker_orders(),
            Message::WalletTrackerRefreshOrders(address) => {
                if self.wallet_tracker.tracked_addresses.contains(&address) {
                    self.queue_wallet_tracker_order_refresh(address);
                    return self.refresh_next_wallet_tracker_orders();
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }
}
