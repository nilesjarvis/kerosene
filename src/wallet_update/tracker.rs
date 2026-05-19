mod entries;
mod refresh;
mod results;
mod window;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_wallet_tracker_list(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWalletTrackerWindow => self.open_wallet_tracker_window(),
            message @ (Message::WalletTrackerInputChanged(_)
            | Message::WalletTrackerLabelInputChanged(_)
            | Message::WalletTrackerAdd
            | Message::WalletTrackerMute(_)
            | Message::WalletTrackerUnmute(_)
            | Message::WalletTrackerRemove(_)
            | Message::WalletTrackerLabelChanged(_, _)) => {
                self.update_wallet_tracker_entries(message)
            }
            message @ (Message::WalletTrackerRefresh
            | Message::WalletTrackerRefreshDue
            | Message::WalletTrackerRefreshOne(_)
            | Message::WalletTrackerRefreshOrdersDue
            | Message::WalletTrackerRefreshOrders(_)) => {
                self.update_wallet_tracker_refresh(message)
            }
            message @ (Message::WalletTrackerLoaded(_, _)
            | Message::WalletTrackerOrdersLoaded(_, _)) => {
                self.apply_wallet_tracker_results(message)
            }
            _ => Task::none(),
        }
    }
}
