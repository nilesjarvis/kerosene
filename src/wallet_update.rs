use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod compact;
mod details;
mod remote_database;
mod tracker;

impl TradingTerminal {
    pub(crate) fn update_wallet_tracker(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::RemoteWalletDatabaseUrlChanged(_)
            | Message::SaveRemoteWalletDatabase
            | Message::DisconnectRemoteWalletDatabase
            | Message::RemoteWalletDatabaseSync
            | Message::RemoteWalletDatabaseLoaded(_, _)) => {
                return self.update_remote_wallet_database(message);
            }
            message @ (Message::CompactWalletSelected(_, _)
            | Message::CompactWalletBack(_)
            | Message::CompactWalletRefresh(_)
            | Message::CompactWalletDetailsLoaded(_, _, _, _)) => {
                return self.update_compact_wallet_tracker(message);
            }
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
