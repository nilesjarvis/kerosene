use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Subscription;

// ---------------------------------------------------------------------------
// Portfolio And Income Timers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_analytics_timer_subscriptions(&self, subs: &mut Vec<Subscription<Message>>) {
        let has_income_pane = self.pane_is_open(|kind| matches!(kind, PaneKind::Income));
        let income_poll_enabled = has_income_pane
            && self.connected_address.is_some()
            && self
                .connected_order_account_snapshot()
                .is_some_and(|(_, data)| data.is_portfolio_margin());
        if income_poll_enabled {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(30))
                    .map(|_| Message::RefreshIncome),
            );
        }

        let has_portfolio_pane = self.pane_is_open(|kind| matches!(kind, PaneKind::Portfolio));
        if has_portfolio_pane && self.connected_address.is_some() {
            let has_live_positions =
                self.connected_order_account_snapshot()
                    .is_some_and(|(_, data)| {
                        data.clearinghouse.asset_positions.iter().any(|position| {
                            !self.symbol_key_is_hidden(&position.position.coin)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_portfolio_pane_enables_analytics_polling() {
        let (mut terminal, _) =
            TradingTerminal::boot_from_config(crate::config::KeroseneConfig::default());
        terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
        terminal.insert_test_canvas_pane(7, PaneKind::Portfolio);
        let mut subscriptions = Vec::new();

        terminal.push_analytics_timer_subscriptions(&mut subscriptions);

        assert_eq!(subscriptions.len(), 1);
    }
}
