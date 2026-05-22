use crate::account::{AccountData, AccountDataFetchScope, fetch_account_data_scoped};
use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    fn account_refresh_backoff_remaining_ms(&self) -> Option<u64> {
        let until_ms = self.account_refresh_backoff_until_ms?;
        let now_ms = Self::now_ms();
        (until_ms > now_ms).then_some(until_ms - now_ms)
    }

    fn account_refresh_rate_limited(error: &str) -> bool {
        let error = error.to_ascii_lowercase();
        error.contains("429") || error.contains("too many requests") || error.contains("rate limit")
    }

    fn account_refresh_backoff_message(remaining_ms: u64) -> String {
        format!(
            "Account refresh is rate limited; retrying in {}s",
            remaining_ms.div_ceil(1000)
        )
    }

    pub(crate) fn account_data_fetch_scope(&self) -> AccountDataFetchScope {
        self.market_universe
            .selected_hip3_dex()
            .map(AccountDataFetchScope::hip3_dex)
            .unwrap_or_else(|| {
                AccountDataFetchScope::all_markets(
                    self.visible_mids_dexes()
                        .into_iter()
                        .filter(|dex| !dex.is_empty()),
                )
            })
    }

    pub(super) fn connect_wallet(&mut self) -> Task<Message> {
        let Some(addr) = Self::normalize_wallet_address(&self.wallet_address_input) else {
            if !self.wallet_address_input.trim().is_empty() {
                self.connected_address = None;
                self.account_data = None;
                self.account_loading = false;
                self.account_reconciliation_required = false;
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
        let stop_chase_ids: Vec<u64> = self
            .chase_orders
            .iter()
            .filter_map(|(id, chase)| {
                (!chase.stop_requested && chase.account_address.as_str() != addr.as_str())
                    .then_some(*id)
            })
            .collect();
        let stop_chase_task = Task::batch(stop_chase_ids.into_iter().map(|id| {
            self.stop_chase_by_id_with_reason(id, "Chase stopped: wallet address changed", false)
        }));
        let stop_twap_ids: Vec<u64> = self
            .twap_orders
            .iter()
            .filter_map(|(id, twap)| {
                (!twap.stop_requested && twap.account_address.as_str() != addr.as_str())
                    .then_some(*id)
            })
            .collect();
        for id in stop_twap_ids {
            let _ = self.stop_twap_with_reason(id, "TWAP stopped: wallet address changed", false);
        }
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
        self.account_reconciliation_required = false;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
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
        let account_scope = self.account_data_fetch_scope();
        let account_task = Task::perform(
            fetch_account_data_scoped(addr.clone(), account_scope),
            move |r| Message::AccountDataLoaded(account_addr.clone(), Box::new(r)),
        );
        let mut tasks = vec![account_task];
        tasks.push(stop_chase_task);
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
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        let stop_chase_ids: Vec<u64> = self
            .chase_orders
            .iter()
            .filter_map(|(id, chase)| (!chase.stop_requested).then_some(*id))
            .collect();
        let stop_chase_task = Task::batch(stop_chase_ids.into_iter().map(|id| {
            self.stop_chase_by_id_with_reason(id, "Chase stopped: wallet disconnected", false)
        }));
        let stop_twap_ids: Vec<u64> = self
            .twap_orders
            .iter()
            .filter_map(|(id, twap)| (!twap.stop_requested).then_some(*id))
            .collect();
        for id in stop_twap_ids {
            let _ = self.stop_twap_with_reason(id, "TWAP stopped: wallet disconnected", false);
        }
        self.connected_address = None;
        self.account_data = None;
        self.account_loading = false;
        self.account_reconciliation_required = false;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
        self.wallet_key_input.zeroize();
        self.wallet_address_input.clear();
        for instance in self.charts.values_mut() {
            instance.chart.active_position = None;
            instance.chart.active_orders.clear();
            instance.chart.trade_markers.clear();
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
        stop_chase_task
    }

    pub(crate) fn apply_account_data_loaded(
        &mut self,
        address: String,
        result: Result<AccountData, String>,
    ) -> Task<Message> {
        if self.connected_address.as_deref() != Some(address.as_str()) {
            if let Ok(data) = result {
                self.reconcile_twap_fills_for_account(&address, &data.fills);
            }
            return Task::none();
        }
        self.account_loading = false;
        match result {
            Ok(data) => {
                self.account_refresh_backoff_until_ms = None;
                self.account_reconciliation_required = false;
                let data = self.filter_account_data_for_muted_tickers(data);
                let is_pm = data.is_portfolio_margin();
                self.account_data = Some(data);
                self.account_error = None;
                self.sync_all_chart_overlays();
                let chase_task = self.reconcile_chase_after_account_refresh();
                self.reconcile_twap_fills_from_account();

                let income_pane_open = self
                    .panes
                    .iter()
                    .any(|(_, kind)| matches!(kind, PaneKind::Income));

                if is_pm && income_pane_open {
                    self.income.loading = true;
                    let income_task = Task::perform(fetch_income_data(address.clone()), move |r| {
                        Message::IncomeLoaded(address.clone(), Box::new(r))
                    });
                    return Task::batch([chase_task, income_task]);
                }
                return chase_task;
            }
            Err(e) => {
                if Self::account_refresh_rate_limited(&e) {
                    self.account_refresh_backoff_until_ms = Some(Self::now_ms() + 60_000);
                }
                self.account_error = Some(e);
            }
        }
        Task::none()
    }

    pub(crate) fn refresh_account_data(&mut self) -> Task<Message> {
        if let Some(remaining_ms) = self.account_refresh_backoff_remaining_ms() {
            self.account_error = Some(Self::account_refresh_backoff_message(remaining_ms));
            return Task::none();
        }
        if !self.account_loading
            && let Some(addr) = &self.connected_address
        {
            return self.force_refresh_account_data_for_reconciliation(addr.clone());
        }
        Task::none()
    }

    pub(crate) fn force_refresh_account_data_for_reconciliation(
        &mut self,
        addr: String,
    ) -> Task<Message> {
        if self.connected_address.as_deref() != Some(addr.as_str()) {
            return Task::none();
        }
        if let Some(remaining_ms) = self.account_refresh_backoff_remaining_ms() {
            self.account_loading = false;
            self.account_error = Some(Self::account_refresh_backoff_message(remaining_ms));
            return Task::none();
        }
        let requested_addr = addr.clone();
        self.account_loading = true;
        self.account_reconciliation_required = true;
        self.account_error = None;
        let scope = self.account_data_fetch_scope();
        Task::perform(fetch_account_data_scoped(addr, scope), move |r| {
            Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
        })
    }

    pub(crate) fn refresh_account_data_for_twap_reconciliation(
        &mut self,
        addr: String,
    ) -> Task<Message> {
        if self.connected_address.as_deref() == Some(addr.as_str()) {
            return self.force_refresh_account_data_for_reconciliation(addr);
        }

        let requested_addr = addr.clone();
        let scope = self.account_data_fetch_scope();
        Task::perform(fetch_account_data_scoped(addr, scope), move |r| {
            Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
        })
    }
}
