mod journal;
mod menu;
mod widgets;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_panes(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchBottomTab(_)
            | Message::CloseAllMenus
            | Message::ToggleAddWidgetMenu
            | Message::ToggleLayoutMenu
            | Message::ToggleTickerTape
            | Message::SetAddWidgetPlacement(_) => self.update_pane_menu(message),
            Message::AddTradingJournal => self.add_trading_journal_window(),
            Message::AddPositionsHistoryPane
            | Message::AddPortfolioPane
            | Message::AddIncomePane
            | Message::AddCalendarPane
            | Message::AddLiquidationsPane
            | Message::AddLiquidationsDistributionPane
            | Message::AddTrackedTradesPane
            | Message::AddTelegramFeedPane
            | Message::AddXFeedPane
            | Message::AddAdvancedOrdersPane
            | Message::AddOutcomesPane
            | Message::AddHypeEtfsPane
            | Message::AddHypeUnstakingQueuePane => self.add_widget_pane(message),
            _ => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState, UserFeeRates,
    };
    use crate::config::KeroseneConfig;
    use crate::hype_unstaking_state::{HypeUnstakingEvent, HypeUnstakingQueueData};
    use crate::pane_state::PaneKind;
    use std::time::{Duration, Instant};

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn account_data_with_pm_enabled() -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: true,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: TradingTerminal::now_ms(),
        }
    }

    #[test]
    fn positions_history_add_message_reopens_bottom_tabs_pane() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let bottom_tabs = terminal
            .find_pane_matching(|kind| matches!(kind, PaneKind::BottomTabs { .. }))
            .expect("default bottom tabs pane");
        terminal.update_pane_interactions(Message::ClosePane(bottom_tabs));
        terminal.add_widget_menu_open = true;

        let _task = terminal.update_panes(Message::AddPositionsHistoryPane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(
            kind,
            PaneKind::BottomTabs {
                active_tab: crate::account_state::BottomTab::Positions
            }
        )));
    }

    #[test]
    fn portfolio_pane_add_starts_initial_refresh_for_connected_account() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.add_widget_menu_open = true;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        let request_id = terminal.portfolio.refresh_request_id;

        let _task = terminal.update_panes(Message::AddPortfolioPane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::Portfolio)));
        assert!(terminal.portfolio.loading);
        assert_ne!(terminal.portfolio.refresh_request_id, request_id);
    }

    #[test]
    fn income_pane_add_starts_initial_refresh_for_portfolio_margin_account() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.add_widget_menu_open = true;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal
            .set_account_data_for_address_for_test(TEST_ACCOUNT, account_data_with_pm_enabled());
        let request_id = terminal.income.refresh_request_id;

        let _task = terminal.update_panes(Message::AddIncomePane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::Income)));
        assert!(terminal.income.loading);
        assert_ne!(terminal.income.refresh_request_id, request_id);
    }

    #[test]
    fn liquidations_distribution_add_message_opens_pane() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.add_widget_menu_open = true;

        let _task = terminal.update_panes(Message::AddLiquidationsDistributionPane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution)));
    }

    #[test]
    fn telegram_feed_add_message_opens_pane_and_requests_initial_refresh() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.add_widget_menu_open = true;

        let _task = terminal.update_panes(Message::AddTelegramFeedPane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::TelegramFeed)));
        assert_eq!(terminal.telegram_feed.loading_channels, vec!["marketfeed"]);
    }

    #[test]
    fn hype_unstaking_queue_pane_add_refreshes_stale_cached_data() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.add_widget_menu_open = true;
        terminal.hype_unstaking_queue.data =
            Some(HypeUnstakingQueueData::new(vec![HypeUnstakingEvent {
                unlock_time_ms: TradingTerminal::now_ms().saturating_add(60_000),
                user: TEST_ACCOUNT.to_string(),
                amount_wei: 100,
            }]));
        terminal.hype_unstaking_queue.last_fetch =
            Some(Instant::now() - Duration::from_secs(10 * 60));
        let request_id = terminal.hype_unstaking_queue.refresh_request_id;

        let _task = terminal.update_panes(Message::AddHypeUnstakingQueuePane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::HypeUnstakingQueue)));
        assert!(terminal.hype_unstaking_queue.loading);
        assert_ne!(terminal.hype_unstaking_queue.refresh_request_id, request_id);
    }
}
