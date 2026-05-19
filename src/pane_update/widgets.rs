use crate::account::AccountData;
use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_management::AddPaneOutcome;
use crate::pane_state::PaneKind;
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
                    && let Some(addr) = &self.connected_address
                {
                    let requested_addr = addr.clone();
                    self.portfolio.loading = true;
                    return Task::perform(fetch_portfolio_history(addr.clone()), move |r| {
                        Message::PortfolioLoaded(requested_addr.clone(), Box::new(r))
                    });
                }
            }
            Message::AddIncomePane => {
                self.add_widget_menu_open = false;
                let is_pm = self
                    .account_data
                    .as_ref()
                    .is_some_and(AccountData::is_portfolio_margin);
                if let Some(pane) = self.find_pane_matching(|kind| matches!(kind, PaneKind::Income))
                {
                    self.focus = Some(pane);
                    self.push_toast("Income is already open".to_string(), false);
                    if is_pm
                        && self.income.data.is_none()
                        && let Some(addr) = &self.connected_address
                    {
                        let requested_addr = addr.clone();
                        self.income.loading = true;
                        return Task::perform(fetch_income_data(addr.clone()), move |r| {
                            Message::IncomeLoaded(requested_addr.clone(), Box::new(r))
                        });
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
                    && let Some(addr) = &self.connected_address
                {
                    let requested_addr = addr.clone();
                    self.income.loading = true;
                    return Task::perform(fetch_income_data(addr.clone()), move |r| {
                        Message::IncomeLoaded(requested_addr.clone(), Box::new(r))
                    });
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
            _ => {}
        }

        Task::none()
    }
}
