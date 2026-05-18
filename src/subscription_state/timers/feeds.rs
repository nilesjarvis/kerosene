use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Account And HyperDash Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_account_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        if self.connected_address.is_some() {
            let refresh_secs = self
                .account_data_fetch_scope()
                .automatic_refresh_interval_secs();
            subs.push(
                iced::time::every(std::time::Duration::from_secs(refresh_secs))
                    .map(|_| Message::RefreshAccountData),
            );
        }
    }

    pub(super) fn push_hyperdash_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let has_liq_charts = !self.hyperdash_api_key.is_empty()
            && self
                .charts
                .values()
                .any(|inst| inst.show_liquidations && !self.symbol_key_is_hidden(&inst.symbol));
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
                .any(|inst| inst.show_heatmap && !self.symbol_key_is_hidden(&inst.symbol));
        if has_heat_charts {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::RefreshHeatmap),
            );
        }

        let has_positioning_infos = !self.hyperdash_api_key.is_empty()
            && self.panes.iter().any(|(_, kind)| {
                let PaneKind::PositioningInfo(id) = kind else {
                    return false;
                };
                self.positioning_infos.get(id).is_some_and(|instance| {
                    !instance.symbol.trim().is_empty()
                        && !self.symbol_key_is_hidden(&instance.symbol)
                })
            });
        if has_positioning_infos {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::RefreshPositioningInfo),
            );
        }
    }
}
