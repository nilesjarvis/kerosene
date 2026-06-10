mod refresh;

use crate::account::fetch_account_data_scoped_with_provider;
use crate::account_analytics::fetch_portfolio_history;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn connect_wallet(&mut self) -> Task<Message> {
        let Some(addr) = Self::normalize_wallet_address(&self.wallet_address_input) else {
            if !self.wallet_address_input.trim().is_empty() {
                self.connected_address = None;
                self.account_data = None;
                self.pending_order_indicators.clear();
                self.pending_leverage_update = None;
                self.order_leverage_dropdown_open = false;
                self.account_loading = false;
                self.account_refresh_followup_pending = false;
                self.account_reconciliation_required = false;
                self.account_error = Some("Invalid wallet address".to_string());
                self.portfolio.loading = false;
                self.portfolio.data = None;
                self.portfolio.last_error = None;
                self.income.loading = false;
                self.income.data = None;
                self.income.last_error = None;
                self.last_income_alert_time = None;
                for instance in self.charts.values_mut() {
                    instance.chart.clear_hud_armed();
                }
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
                (!chase.lifecycle.is_stopping() && chase.account_address.as_str() != addr.as_str())
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
        self.pending_order_indicators.clear();
        self.pending_leverage_update = None;
        self.order_leverage_dropdown_open = false;
        for instance in self.charts.values_mut() {
            instance.chart.clear_hud_armed();
        }
        self.account_loading = true;
        self.account_refresh_followup_pending = false;
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
        let account_provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key.trim().to_string();
        let account_task = Task::perform(
            fetch_account_data_scoped_with_provider(
                addr.clone(),
                account_scope,
                account_provider,
                hydromancer_key,
            ),
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
            .filter_map(|(id, chase)| (!chase.lifecycle.is_stopping()).then_some(*id))
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
        self.pending_order_indicators.clear();
        self.pending_leverage_update = None;
        self.order_leverage_dropdown_open = false;
        self.account_loading = false;
        self.account_refresh_followup_pending = false;
        self.account_reconciliation_required = false;
        self.account_error = None;
        self.account_refresh_backoff_until_ms = None;
        self.wallet_key_input.zeroize();
        self.wallet_address_input.clear();
        for instance in self.charts.values_mut() {
            instance.chart.clear_hud_armed();
            instance.chart.active_position = None;
            instance.chart.active_orders.clear();
            instance.chart.trade_markers.clear();
            instance.chart.set_pending_market_order_loading([]);
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
}

#[cfg(test)]
mod tests {
    use crate::app_state::TradingTerminal;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn disconnect_clears_pending_indicators_and_market_pulse() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        let pending_id = terminal.add_pending_market_order_placement_indicator(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            true,
            "1".to_string(),
            "100".to_string(),
        );
        assert!(pending_id.is_some());
        assert!(
            terminal
                .charts
                .get(&1)
                .expect("chart")
                .chart
                .hud_order_animation_active()
        );

        let _task = terminal.disconnect_wallet();

        assert!(terminal.pending_order_indicators.is_empty());
        let chart = &terminal.charts.get(&1).expect("chart").chart;
        assert!(chart.active_orders.is_empty());
        assert!(!chart.hud_order_animation_active());
    }
}
