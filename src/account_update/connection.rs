use crate::account::{AccountData, fetch_account_data};
use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn connect_wallet(&mut self) -> Task<Message> {
        let Some(addr) = Self::normalize_wallet_address(&self.wallet_address_input) else {
            if !self.wallet_address_input.trim().is_empty() {
                self.connected_address = None;
                self.account_data = None;
                self.account_loading = false;
                self.account_error = Some("Invalid wallet address".to_string());
                self.portfolio.loading = false;
                self.portfolio.data = None;
                self.portfolio.last_error = None;
                self.income.loading = false;
                self.income.data = None;
                self.income.last_error = None;
                self.last_income_alert_time = None;
                if self.journal.window_id.is_some() {
                    self.journal.clear_active_account_data();
                    self.journal.error = Some("Invalid wallet address".to_string());
                }
                self.sync_all_chart_overlays();
                self.push_toast("Invalid wallet address".to_string(), true);
            }
            return Task::none();
        };

        self.wallet_address_input = addr.clone();
        if self.active_account_is_ghost() {
            self.wallet_key_input.zeroize();
            if let Some(profile) = self.accounts.get_mut(self.active_account_index) {
                profile.agent_key.zeroize();
            }
        } else {
            if let Some(profile) = self.accounts.get_mut(self.active_account_index) {
                profile.wallet_address = addr.clone();
            }
            self.persist_active_profile_secrets();
        }
        self.connected_address = Some(addr.clone());
        self.account_data = None;
        self.account_loading = true;
        self.account_error = None;
        self.portfolio.data = None;
        self.portfolio.last_error = None;
        self.income.loading = false;
        self.income.data = None;
        self.income.last_error = None;
        self.last_income_alert_time = None;
        self.sync_all_chart_overlays();
        self.hyperdash_api_key.zeroize();
        self.hyperdash_api_key = self.hyperdash_key_input.clone();
        self.persist_config();

        let account_addr = addr.clone();
        let account_task = Task::perform(fetch_account_data(addr.clone()), move |r| {
            Message::AccountDataLoaded(account_addr.clone(), Box::new(r))
        });
        let mut tasks = vec![account_task];
        self.portfolio.loading = true;
        let portfolio_addr = addr.clone();
        tasks.push(Task::perform(fetch_portfolio_history(addr), move |r| {
            Message::PortfolioLoaded(portfolio_addr.clone(), Box::new(r))
        }));
        tasks.extend(self.mids_bootstrap_tasks());
        tasks.push(self.load_journal_for_active_account(false));
        Task::batch(tasks)
    }

    pub(super) fn disconnect_wallet(&mut self) -> Task<Message> {
        self.connected_address = None;
        self.account_data = None;
        self.account_loading = false;
        self.account_error = None;
        self.wallet_key_input.zeroize();
        self.wallet_address_input.clear();
        for instance in self.charts.values_mut() {
            instance.chart.active_position = None;
            instance.chart.active_orders.clear();
        }
        self.portfolio.loading = false;
        self.portfolio.data = None;
        self.portfolio.last_error = None;
        self.income.loading = false;
        self.income.data = None;
        self.income.last_error = None;
        self.last_income_alert_time = None;
        if self.journal.window_id.is_some() {
            self.journal.clear_active_account_data();
            self.journal.error = Some("Connect an account before loading the journal.".to_string());
        }
        self.persist_config();
        Task::none()
    }

    pub(super) fn apply_account_data_loaded(
        &mut self,
        address: String,
        result: Result<AccountData, String>,
    ) -> Task<Message> {
        if self.connected_address.as_deref() != Some(address.as_str()) {
            return Task::none();
        }
        self.account_loading = false;
        match result {
            Ok(data) => {
                let data = self.filter_account_data_for_muted_tickers(data);
                let is_pm = data.is_portfolio_margin();
                self.account_data = Some(data);
                self.account_error = None;
                self.sync_all_chart_overlays();

                let income_pane_open = self
                    .panes
                    .iter()
                    .any(|(_, kind)| matches!(kind, PaneKind::Income));

                if is_pm && income_pane_open {
                    self.income.loading = true;
                    return Task::perform(fetch_income_data(address.clone()), move |r| {
                        Message::IncomeLoaded(address.clone(), Box::new(r))
                    });
                }
            }
            Err(e) => {
                self.account_error = Some(e);
            }
        }
        Task::none()
    }

    pub(crate) fn refresh_account_data(&mut self) -> Task<Message> {
        if !self.account_loading
            && let Some(addr) = &self.connected_address
        {
            let addr = addr.clone();
            let requested_addr = addr.clone();
            self.account_loading = true;
            self.account_error = None;
            return Task::perform(fetch_account_data(addr), move |r| {
                Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
            });
        }
        Task::none()
    }
}
