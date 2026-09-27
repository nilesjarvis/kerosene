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
        if !self.wallet_tracker.remote_database.url.is_empty() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(
                    crate::wallet_state::remote_database::SYNC_INTERVAL_SECS,
                ))
                .map(|_| Message::RemoteWalletDatabaseSync),
            );
        }
        if self.wallet_tracker_is_visible() && !self.wallet_tracker.tracked_addresses.is_empty() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(
                    WALLET_TRACKER_CORE_TICK_SECS,
                ))
                .map(|_| Message::WalletTrackerRefreshDue),
            );
            if self.wallet_tracker.window_id.is_some() {
                subs.push(
                    iced::time::every(std::time::Duration::from_secs(
                        WALLET_TRACKER_ORDER_TICK_SECS,
                    ))
                    .map(|_| Message::WalletTrackerRefreshOrdersDue),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_state::PaneKind;

    #[test]
    fn remote_wallet_sync_runs_with_no_wallets_or_visible_tracker() {
        let config = crate::config::KeroseneConfig {
            remote_wallet_database: crate::config::RemoteWalletDatabaseConfig {
                url: "http://wallets.test".into(),
            },
            ..Default::default()
        };
        let mut terminal = TradingTerminal::boot_from_config(config).0;
        terminal.panes = iced::widget::pane_grid::State::new(PaneKind::Watchlist).0;
        let mut subscriptions = Vec::new();
        terminal.push_wallet_tracker_timer_subscriptions(&mut subscriptions);
        assert_eq!(subscriptions.len(), 1);
        let _ = terminal.update(Message::DisconnectRemoteWalletDatabase);
        subscriptions.clear();
        terminal.push_wallet_tracker_timer_subscriptions(&mut subscriptions);
        assert!(subscriptions.is_empty());
    }

    #[test]
    fn compact_wallet_timer_runs_without_tracker_window_and_skips_order_counts() {
        let mut terminal =
            TradingTerminal::boot_from_config(crate::config::KeroseneConfig::default()).0;
        terminal.panes = iced::widget::pane_grid::State::new(PaneKind::CompactWalletTracker(0)).0;
        terminal.wallet_tracker.tracked_addresses =
            vec!["0xabc0000000000000000000000000000000000000".into()];
        let subscriptions = |terminal: &TradingTerminal| {
            let mut subs = Vec::new();
            terminal.push_wallet_tracker_timer_subscriptions(&mut subs);
            subs.len()
        };
        assert_eq!(subscriptions(&terminal), 1);
        terminal.wallet_tracker.window_id = Some(iced::window::Id::unique());
        assert_eq!(subscriptions(&terminal), 2);
        terminal.wallet_tracker.window_id = None;
        terminal.panes = iced::widget::pane_grid::State::new(PaneKind::Watchlist).0;
        assert_eq!(subscriptions(&terminal), 0);
        terminal.panes = iced::widget::pane_grid::State::new(PaneKind::CompactWalletTracker(0)).0;
        terminal.wallet_tracker.tracked_addresses.clear();
        assert_eq!(subscriptions(&terminal), 0);
    }
}
