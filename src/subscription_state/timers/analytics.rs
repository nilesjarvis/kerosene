use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Portfolio And Income Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_analytics_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let has_income_pane = self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Income));
        let income_poll_enabled = has_income_pane
            && self.connected_address.is_some()
            && self
                .account_data
                .as_ref()
                .is_some_and(AccountData::is_portfolio_margin);
        if income_poll_enabled {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(30))
                    .map(|_| Message::RefreshIncome),
            );
        }

        let has_portfolio_pane = self
            .panes
            .iter()
            .any(|(_, kind)| matches!(kind, PaneKind::Portfolio));
        if has_portfolio_pane && self.connected_address.is_some() {
            let has_live_positions = self.account_data.as_ref().is_some_and(|data| {
                data.clearinghouse.asset_positions.iter().any(|position| {
                    !self.is_ticker_muted(&position.position.coin)
                        && position
                            .position
                            .szi
                            .parse::<f64>()
                            .ok()
                            .is_some_and(|size| size.abs() > 1e-9)
                })
            });
            let poll_secs = if has_live_positions { 5 } else { 30 };
            subs.push(
                iced::time::every(std::time::Duration::from_secs(poll_secs))
                    .map(|_| Message::RefreshPortfolio),
            );
        }
    }
}
