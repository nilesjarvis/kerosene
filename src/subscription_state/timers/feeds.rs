use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Account And HyperDash Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_account_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self.connected_address.is_some() {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(30))
                    .map(|_| Message::RefreshAccountData),
            );
        }
    }

    pub(super) fn push_hyperdash_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let has_liq_charts = !self.hyperdash_api_key.is_empty()
            && self
                .charts
                .values()
                .any(|inst| inst.show_liquidations && !self.is_ticker_muted(&inst.symbol));
        if has_liq_charts {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::RefreshLiquidations),
            );
        }

        let has_heat_charts = !self.hyperdash_api_key.is_empty()
            && self
                .charts
                .values()
                .any(|inst| inst.show_heatmap && !self.is_ticker_muted(&inst.symbol));
        if has_heat_charts {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::RefreshHeatmap),
            );
        }
    }
}
