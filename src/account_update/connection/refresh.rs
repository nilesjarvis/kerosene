use crate::account::{
    AccountData, AccountDataFetchScope, dedupe_user_fills_preserving_order,
    fetch_account_data_scoped_with_provider,
};
use crate::account_analytics::fetch_income_data;
use crate::app_state::TradingTerminal;
use crate::journal;
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
            if let Ok(mut data) = result {
                data.fills = dedupe_user_fills_preserving_order(data.fills);
                self.reconcile_twap_fills_for_account(&address, &data.fills);
            }
            return Task::none();
        }
        self.account_loading = false;
        let followup_pending = std::mem::take(&mut self.account_refresh_followup_pending);
        match result {
            Ok(mut data) => {
                self.account_refresh_backoff_until_ms = None;
                self.account_reconciliation_required = false;
                data.fills = dedupe_user_fills_preserving_order(data.fills);
                let is_pm = data.is_portfolio_margin();
                self.account_data = Some(data);
                let position_reconciliation =
                    self.reconcile_journal_current_positions_from_account();
                if position_reconciliation.added_open_positions > 0 {
                    self.push_journal_warning_message(journal::current_position_fallback_warning(
                        position_reconciliation.added_open_positions,
                    ));
                }
                self.sync_order_leverage_form_for_active_symbol();
                self.account_error = None;
                self.sync_all_chart_overlays();
                let chase_task = self.reconcile_chase_after_account_refresh();
                self.reconcile_twap_fills_from_account();
                // A refresh was requested while this fetch was in flight, so
                // this snapshot predates whatever prompted it; run the queued
                // follow-up now.
                let followup_task = if followup_pending {
                    self.force_refresh_account_data_for_reconciliation(address.clone())
                } else {
                    Task::none()
                };

                let income_pane_open = self
                    .panes
                    .iter()
                    .any(|(_, kind)| matches!(kind, PaneKind::Income));

                if is_pm && income_pane_open {
                    self.income.loading = true;
                    let income_task = Task::perform(fetch_income_data(address.clone()), move |r| {
                        Message::IncomeLoaded(address.clone(), Box::new(r))
                    });
                    return Task::batch([chase_task, income_task, followup_task]);
                }
                return Task::batch([chase_task, followup_task]);
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
        if let Some(addr) = &self.connected_address {
            if self.account_loading {
                // The fetch already in flight predates whatever prompted this
                // refresh (e.g. an order ack), so its snapshot won't reflect
                // it. Queue one follow-up instead of silently dropping the
                // request — otherwise a confirmed order can vanish from the
                // Orders tab until the next periodic refresh.
                self.account_refresh_followup_pending = true;
                return Task::none();
            }
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
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key.trim().to_string();
        self.account_loading = true;
        self.account_reconciliation_required = true;
        self.account_error = None;
        Task::perform(
            fetch_account_data_scoped_with_provider(addr, scope, provider, hydromancer_key),
            move |r| Message::AccountDataLoaded(requested_addr.clone(), Box::new(r)),
        )
    }

    pub(crate) fn refresh_account_data_for_twap_reconciliation(
        &mut self,
        addr: String,
    ) -> Task<Message> {
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key.trim().to_string();
        if self.connected_address.as_deref() == Some(addr.as_str()) {
            return self.force_refresh_account_data_for_reconciliation(addr);
        }

        let requested_addr = addr.clone();
        let scope = self.account_data_fetch_scope();
        Task::perform(
            fetch_account_data_scoped_with_provider(addr, scope, provider, hydromancer_key),
            move |r| Message::AccountDataLoaded(requested_addr.clone(), Box::new(r)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary, Position,
        PositionLeverage, SpotClearinghouseState, UserFeeRates,
    };

    fn account_data_with_position(coin: &str) -> AccountData {
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
                asset_positions: vec![AssetPosition {
                    position: Position {
                        coin: coin.to_string(),
                        szi: "1".to_string(),
                        entry_px: "100".to_string(),
                        position_value: "100".to_string(),
                        unrealized_pnl: "0".to_string(),
                        liquidation_px: None,
                        leverage: PositionLeverage {
                            leverage_type: "cross".to_string(),
                            value: 1,
                        },
                        margin_used: "0".to_string(),
                        cum_funding: None,
                    },
                    liquidation_px: None,
                }],
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
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: crate::app_time::now_ms(),
        }
    }

    #[test]
    fn expired_account_refresh_backoff_does_not_overflow() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.account_refresh_backoff_until_ms = Some(1);

        assert_eq!(terminal.account_refresh_backoff_remaining_ms(), None);
    }

    #[test]
    fn account_refresh_keeps_hidden_positions_in_stored_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.muted_tickers.insert("HYPE".to_string());

        let _task =
            terminal.apply_account_data_loaded(address, Ok(account_data_with_position("HYPE")));

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "HYPE");
    }

    #[test]
    fn refresh_requested_mid_fetch_is_queued_and_runs_after_load() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = true;

        // The in-flight fetch predates this request (e.g. an order ack), so
        // it must be queued instead of silently dropped.
        let _task = terminal.refresh_account_data();
        assert!(terminal.account_refresh_followup_pending);

        let _task = terminal
            .apply_account_data_loaded(address.clone(), Ok(account_data_with_position("HYPE")));

        // The queued follow-up fires as soon as the stale snapshot lands.
        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_loading);
    }

    #[test]
    fn refresh_without_in_flight_fetch_is_not_queued() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = false;

        let _task = terminal.refresh_account_data();

        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_loading);

        let _task = terminal
            .apply_account_data_loaded(address.clone(), Ok(account_data_with_position("HYPE")));
        assert!(!terminal.account_loading);
    }
}
