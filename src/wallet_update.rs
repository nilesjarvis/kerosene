use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod details;
mod tracker;

impl TradingTerminal {
    pub(crate) fn update_wallet_tracker(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::OpenWalletDetailsWindow(_)
            | Message::RefreshWalletDetails(_)
            | Message::WalletDetailsLoaded(_, _, _, _)
            | Message::WalletDetailsWsUpdate(_, _)) => return self.update_wallet_details(message),
            message @ (Message::OpenWalletTrackerWindow
            | Message::WalletTrackerInputChanged(_)
            | Message::WalletTrackerLabelInputChanged(_)
            | Message::WalletTrackerAdd
            | Message::WalletTrackerMute(_)
            | Message::WalletTrackerUnmute(_)
            | Message::WalletTrackerRemove(_)
            | Message::WalletTrackerLabelChanged(_, _)
            | Message::WalletTrackerRefresh
            | Message::WalletTrackerRefreshDue
            | Message::WalletTrackerRefreshOne(_)
            | Message::WalletTrackerRefreshOrdersDue
            | Message::WalletTrackerRefreshOrders(_)
            | Message::WalletTrackerLoaded(_, _, _)
            | Message::WalletTrackerBatchLoaded(_, _)
            | Message::WalletTrackerOrdersLoaded(_, _, _)) => {
                return self.update_wallet_tracker_list(message);
            }
            _ => {}
        }

        Task::none()
    }
}
