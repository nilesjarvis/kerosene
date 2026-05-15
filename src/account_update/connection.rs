use crate::account::{AccountData, AccountDataFetchScope, fetch_account_data_scoped};
use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    fn account_data_refresh_task(&mut self, addr: String) -> Task<Message> {
        let requested_addr = addr.clone();
        let scope = self.account_data_fetch_scope();
        self.account_loading = true;
        self.account_refresh_requested_scope = Some(scope.clone());
        self.account_error = None;
        Task::perform(fetch_account_data_scoped(addr, scope), move |r| {
            Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
        })
    }

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
                self.account_refresh_requested_scope = None;
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

        let account_task = self.account_data_refresh_task(addr.clone());
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
        self.account_refresh_requested_scope = None;
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
        let current_scope = self.account_data_fetch_scope();
        if self
            .account_refresh_requested_scope
            .as_ref()
            .is_some_and(|requested_scope| requested_scope != &current_scope)
        {
            self.account_loading = false;
            self.account_refresh_requested_scope = None;
            self.account_error = Some(
                "Discarded account refresh from the previous market universe; refreshing current scope"
                    .to_string(),
            );
            return self.force_refresh_account_data_for_reconciliation(address);
        }

        self.account_loading = false;
        self.account_refresh_requested_scope = None;
        match result {
            Ok(data) => {
                if data.fetch_scope != current_scope {
                    self.account_error = Some(
                        "Discarded account data from the previous market universe; refreshing current scope"
                            .to_string(),
                    );
                    return self.force_refresh_account_data_for_reconciliation(address);
                }
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
        self.account_reconciliation_required = true;
        let task = self.account_data_refresh_task(addr);
        debug_assert!(self.connected_address.as_deref() == Some(requested_addr.as_str()));
        task
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountAbstractionMode, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState,
    };
    use crate::config::MarketUniverseConfig;

    fn account_data_with_scope(fetch_scope: AccountDataFetchScope) -> AccountData {
        AccountData {
            fetch_scope,
            request_weight_estimate: 0,
            account_abstraction: AccountAbstractionMode::default(),
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
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: Default::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    #[test]
    fn account_data_loaded_discards_stale_market_universe_scope_and_refreshes() {
        let (mut terminal, _) = TradingTerminal::boot();
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        let old_scope = AccountDataFetchScope::hip3_dex("dex-a");

        terminal.connected_address = Some(address.clone());
        terminal.market_universe = MarketUniverseConfig::hip3_dex("dex-a");
        terminal.account_loading = true;
        terminal.account_refresh_requested_scope = Some(old_scope.clone());
        terminal.market_universe = MarketUniverseConfig::All;

        let _task =
            terminal.apply_account_data_loaded(address, Ok(account_data_with_scope(old_scope)));

        assert!(terminal.account_data.is_none());
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert_eq!(
            terminal.account_refresh_requested_scope,
            Some(terminal.account_data_fetch_scope())
        );
    }

    #[test]
    fn account_data_loaded_accepts_current_market_universe_scope() {
        let (mut terminal, _) = TradingTerminal::boot();
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        let scope = AccountDataFetchScope::hip3_dex("dex-a");

        terminal.connected_address = Some(address.clone());
        terminal.market_universe = MarketUniverseConfig::hip3_dex("dex-a");
        terminal.account_loading = true;
        terminal.account_refresh_requested_scope = Some(scope.clone());

        let _task =
            terminal.apply_account_data_loaded(address, Ok(account_data_with_scope(scope.clone())));

        assert!(!terminal.account_loading);
        assert!(!terminal.account_reconciliation_required);
        assert_eq!(terminal.account_refresh_requested_scope, None);
        assert_eq!(
            terminal.account_data.as_ref().map(|data| &data.fetch_scope),
            Some(&scope)
        );
    }
}
