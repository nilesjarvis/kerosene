use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_state::{WALLET_TRACKER_CORE_TICK_SECS, WALLET_TRACKER_ORDER_TICK_SECS};

use iced::Subscription;

// ---------------------------------------------------------------------------
// Wallet Tracker Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_wallet_tracker_timer_subscriptions(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if self.wallet_tracker.window_id.is_some()
            && !self.wallet_tracker.tracked_addresses.is_empty()
        {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(
                    WALLET_TRACKER_CORE_TICK_SECS,
                ))
                .map(|_| Message::WalletTrackerRefreshDue),
            );
            subs.push(
                iced::time::every(std::time::Duration::from_secs(
                    WALLET_TRACKER_ORDER_TICK_SECS,
                ))
                .map(|_| Message::WalletTrackerRefreshOrdersDue),
            );
        }
    }
}
