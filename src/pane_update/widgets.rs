use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_management::AddPaneOutcome;
use crate::pane_state::PaneKind;
use crate::x_feed::{XFeedInstance, XFeedSource};
use iced::Task;

impl TradingTerminal {
    pub(super) fn add_widget_pane(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddPortfolioPane => {
                self.add_widget_menu_open = false;
                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::Portfolio,
                    "Portfolio",
                    |kind| matches!(kind, PaneKind::Portfolio),
                );

                if !matches!(outcome, AddPaneOutcome::Failed)
                    && self.portfolio.data.is_none()
                    && let Some(addr) = self.connected_address.clone()
                {
                    return self.start_portfolio_refresh_for_address(addr);
                }
            }
            Message::AddIncomePane => {
                self.add_widget_menu_open = false;
                let is_pm = self
                    .connected_order_account_snapshot()
                    .is_some_and(|(_, data)| data.is_portfolio_margin());
                if let Some(pane) = self.find_pane_matching(|kind| matches!(kind, PaneKind::Income))
                {
                    self.focus = Some(pane);
                    self.push_toast("Income is already open".to_string(), false);
                    if is_pm
                        && self.income.data.is_none()
                        && let Some(addr) = self.connected_address.clone()
                    {
                        return self.start_income_refresh_for_address(addr);
                    }
                    return Task::none();
                }

                if !is_pm {
                    self.push_toast(
                        "Income widget is available in Portfolio Margin mode only".to_string(),
                        true,
                    );
                    return Task::none();
                }

                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::Income,
                    "Income",
                    |kind| matches!(kind, PaneKind::Income),
                );

                if !matches!(outcome, AddPaneOutcome::Failed)
                    && self.income.data.is_none()
                    && let Some(addr) = self.connected_address.clone()
                {
                    return self.start_income_refresh_for_address(addr);
                }
            }
            Message::AddCalendarPane => {
                self.add_widget_menu_open = false;
                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::Calendar,
                    "Calendar",
                    |kind| matches!(kind, PaneKind::Calendar),
                );
                if !matches!(outcome, AddPaneOutcome::Failed) {
                    return self.request_calendar_refresh(false);
                }
            }
            Message::AddLiquidationsPane => {
                self.add_widget_menu_open = false;
                self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::Liquidations,
                    "Liquidations",
                    |kind| matches!(kind, PaneKind::Liquidations),
                );
            }
            Message::AddLiquidationsDistributionPane => {
                self.add_widget_menu_open = false;
                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::LiquidationsDistribution,
                    "Liquidations Distribution",
                    |kind| matches!(kind, PaneKind::LiquidationsDistribution),
                );
                if !matches!(outcome, AddPaneOutcome::Failed) {
                    return self.request_liquidation_distribution_refresh(false);
                }
            }
            Message::AddAdvancedOrdersPane => {
                self.add_widget_menu_open = false;
                self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::AdvancedOrders,
                    "Advanced Orders",
                    |kind| matches!(kind, PaneKind::AdvancedOrders),
                );
            }
            Message::AddTrackedTradesPane => {
                self.add_widget_menu_open = false;
                self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::TrackedTrades,
                    "Wallet Tracker",
                    |kind| matches!(kind, PaneKind::TrackedTrades),
                );
            }
            Message::AddTelegramFeedPane => {
                self.add_widget_menu_open = false;
                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::TelegramFeed,
                    "Telegram Feed",
                    |kind| matches!(kind, PaneKind::TelegramFeed),
                );
                if !matches!(outcome, AddPaneOutcome::Failed) && self.telegram_feed.posts.is_empty()
                {
                    return self.request_telegram_feed_refresh();
                }
            }
            Message::AddXFeedPane => {
                self.add_widget_menu_open = false;
                let mut id = 0;
                while self.x_feed.instances.contains_key(&id)
                    || self.panes.iter().any(
                        |(_, kind)| matches!(kind, PaneKind::XFeed(existing) if *existing == id),
                    )
                {
                    id = id.saturating_add(1);
                }

                if self
                    .add_pane_next_to_focus(self.add_widget_axis(), PaneKind::XFeed(id), "X Feed")
                    .is_some()
                {
                    self.x_feed
                        .instances
                        .insert(id, XFeedInstance::new(id, XFeedSource::Following));
                    self.persist_config();
                    return self.request_x_feed_refresh(id, true);
                }
            }
            Message::AddOutcomesPane => {
                self.add_widget_menu_open = false;
                self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::Outcomes,
                    "Outcomes",
                    |kind| matches!(kind, PaneKind::Outcomes),
                );
            }
            Message::AddHypeEtfsPane => {
                self.add_widget_menu_open = false;
                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::HypeEtfs,
                    "HYPE ETFs",
                    |kind| matches!(kind, PaneKind::HypeEtfs),
                );
                if !matches!(outcome, AddPaneOutcome::Failed) && self.hype_etfs.data.is_none() {
                    return self.request_hype_etfs_refresh(false);
                }
            }
            Message::AddHypeUnstakingQueuePane => {
                self.add_widget_menu_open = false;
                let outcome = self.add_or_focus_singleton_pane(
                    self.add_widget_axis(),
                    PaneKind::HypeUnstakingQueue,
                    "HYPE Unstaking Queue",
                    |kind| matches!(kind, PaneKind::HypeUnstakingQueue),
                );
                if !matches!(outcome, AddPaneOutcome::Failed) {
                    return self.request_hype_unstaking_queue_refresh(false);
                }
            }
            _ => {}
        }

        Task::none()
    }
}
