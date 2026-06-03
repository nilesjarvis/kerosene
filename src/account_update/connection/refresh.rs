use crate::account::{AccountData, AccountDataFetchScope, fetch_account_data_scoped};
use crate::account_analytics::fetch_income_data;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;

// ---------------------------------------------------------------------------
// Account Refresh
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn account_refresh_backoff_remaining_ms(&self) -> Option<u64> {
        let until_ms = self.account_refresh_backoff_until_ms?;
        let now_ms = Self::now_ms();
        until_ms
            .checked_sub(now_ms)
            .filter(|remaining_ms| *remaining_ms > 0)
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
                self.sync_order_leverage_form_for_active_symbol();
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
                    self.account_refresh_backoff_until_ms =
                        Some(Self::now_ms().saturating_add(60_000));
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
        let scope = self.account_data_fetch_scope();
        self.force_refresh_account_data_with_scope(addr, scope)
    }

    pub(crate) fn force_refresh_account_data_with_scope(
        &mut self,
        addr: String,
        scope: AccountDataFetchScope,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_account_refresh_backoff_does_not_overflow() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.account_refresh_backoff_until_ms = Some(1);

        assert_eq!(terminal.account_refresh_backoff_remaining_ms(), None);
    }
}
