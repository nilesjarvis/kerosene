use crate::account::{
    AccountData, AccountDataFetchScope, AccountDataSection, dedupe_user_fills_preserving_order,
    fetch_account_data_scoped_with_provider,
};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::journal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::read_data_provider::{AccountDataRequestContext, AccountDataRequestScope};

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

    fn schedule_account_refresh_backoff_retry(&mut self, due_ms: u64) -> Task<Message> {
        if self.account_refresh_retry_due_ms == Some(due_ms) {
            return Task::none();
        }

        self.account_refresh_retry_due_ms = Some(due_ms);
        let delay_ms = due_ms.saturating_sub(Self::now_ms()).max(1);
        Task::perform(
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                due_ms
            },
            Message::AccountRefreshBackoffElapsed,
        )
    }

    fn mark_account_reconciliation_waiting_for_backoff(
        &mut self,
        due_ms: u64,
        remaining_ms: u64,
    ) -> Task<Message> {
        self.account_reconciliation_required = true;
        self.account_error = Some(Self::account_refresh_backoff_message(remaining_ms));
        self.schedule_account_refresh_backoff_retry(due_ms)
    }

    pub(crate) fn handle_account_refresh_backoff_elapsed(&mut self, due_ms: u64) -> Task<Message> {
        if self.account_refresh_retry_due_ms != Some(due_ms) {
            return Task::none();
        }

        self.account_refresh_retry_due_ms = None;
        if self.account_refresh_backoff_until_ms != Some(due_ms) {
            return Task::none();
        }
        if self.account_loading {
            return Task::none();
        }

        self.refresh_account_data()
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
        context: AccountDataRequestContext,
        result: Result<AccountData, String>,
    ) -> Task<Message> {
        if matches!(
            context.scope,
            AccountDataRequestScope::TwapReconciliation { .. }
        ) {
            if !self.account_data_request_generation_is_current(&address, context) {
                return Task::none();
            }
            if !self.read_data_request_context_is_current(context.read_data) {
                return self.handle_twap_reconciliation_account_data_failure(
                    &address,
                    "read data provider changed before TWAP reconciliation completed".to_string(),
                );
            }
            match result {
                Ok(mut data) => {
                    if let Some(warning) = fills_incomplete_warning(&data) {
                        return self
                            .handle_twap_reconciliation_account_data_failure(&address, warning);
                    }
                    data.fills = dedupe_user_fills_preserving_order(data.fills);
                    self.reconcile_twap_fills_for_account_after_refresh(&address, &data.fills);
                }
                Err(error) => {
                    return self.handle_twap_reconciliation_account_data_failure(&address, error);
                }
            }
            return Task::none();
        }

        if !self.read_data_request_context_is_current(context.read_data) {
            return self.handle_stale_account_data_loaded(address);
        }
        if !self.account_data_request_generation_is_current(&address, context) {
            return Task::none();
        }
        if self.connected_address.as_deref() != Some(address.as_str()) {
            if let Ok(mut data) = result {
                data.fills = dedupe_user_fills_preserving_order(data.fills);
                self.reconcile_twap_fills_for_account_after_refresh(&address, &data.fills);
            }
            return Task::none();
        }
        self.account_loading = false;
        let followup_pending = std::mem::take(&mut self.account_refresh_followup_pending);
        match result {
            Ok(mut data) => {
                self.account_refresh_backoff_until_ms = None;
                self.account_refresh_retry_due_ms = None;
                self.account_reconciliation_required = followup_pending;
                let fills_incomplete = fills_incomplete_warning(&data);
                data.fills = dedupe_user_fills_preserving_order(data.fills);
                let fills_for_twap = data.fills.clone();
                let is_pm = data.is_portfolio_margin();
                self.bump_account_data_revision();
                self.account_data = Some(data);
                self.account_data_address = Some(address.clone());
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
                let income_pane_open = self
                    .panes
                    .iter()
                    .any(|(_, kind)| matches!(kind, PaneKind::Income));
                // This snapshot predates a refresh request that arrived while
                // it was in flight. Store it for display, but do not let it
                // drive exchange-emitting automation reconciliation.
                let followup_task = if followup_pending {
                    self.force_refresh_account_data_for_reconciliation(address.clone())
                } else {
                    Task::none()
                };
                if followup_pending {
                    if is_pm && income_pane_open {
                        let income_task = self.start_income_refresh_for_address(address.clone());
                        return Task::batch([income_task, followup_task]);
                    }
                    return followup_task;
                }
                self.clear_pending_one_shot_status_request_for_account(&address);
                self.clear_pending_order_status_requests_for_account_after_refresh(&address);
                let chase_task = self.reconcile_chase_after_account_refresh();
                let twap_task = if let Some(warning) = fills_incomplete {
                    self.handle_twap_reconciliation_account_data_failure(&address, warning)
                } else {
                    self.reconcile_twap_fills_for_account_after_refresh(&address, &fills_for_twap);
                    Task::none()
                };

                if is_pm && income_pane_open {
                    let income_task = self.start_income_refresh_for_address(address.clone());
                    return Task::batch([chase_task, twap_task, income_task, followup_task]);
                }
                return Task::batch([chase_task, twap_task, followup_task]);
            }
            Err(e) => {
                if Self::account_refresh_rate_limited(&e) {
                    let due_ms = Self::now_ms().saturating_add(60_000);
                    self.account_refresh_backoff_until_ms = Some(due_ms);
                    return self.mark_account_reconciliation_waiting_for_backoff(due_ms, 60_000);
                }
                self.account_error = Some(redact_sensitive_response_text(&e));
                if followup_pending {
                    return self.force_refresh_account_data_for_reconciliation(address);
                }
            }
        }
        Task::none()
    }

    fn handle_stale_account_data_loaded(&mut self, address: String) -> Task<Message> {
        if self.connected_address.as_deref() != Some(address.as_str()) || !self.account_loading {
            return Task::none();
        }

        self.account_loading = false;
        let followup_pending = std::mem::take(&mut self.account_refresh_followup_pending);
        if followup_pending || self.account_reconciliation_required {
            return self.force_refresh_account_data_for_reconciliation(address);
        }

        Task::none()
    }

    pub(crate) fn refresh_account_data(&mut self) -> Task<Message> {
        if let Some(remaining_ms) = self.account_refresh_backoff_remaining_ms() {
            if self.connected_address.is_some() {
                if let Some(due_ms) = self.account_refresh_backoff_until_ms {
                    return self
                        .mark_account_reconciliation_waiting_for_backoff(due_ms, remaining_ms);
                }
            } else {
                self.account_error = Some(Self::account_refresh_backoff_message(remaining_ms));
            }
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
            if let Some(due_ms) = self.account_refresh_backoff_until_ms {
                return self.mark_account_reconciliation_waiting_for_backoff(due_ms, remaining_ms);
            }
            return Task::none();
        }
        let requested_addr = addr.clone();
        let provider = self.read_data_provider;
        let account_context = self.begin_account_data_request_context();
        let hydromancer_key = self.hydromancer_api_key_for_task();
        self.account_loading = true;
        self.account_reconciliation_required = true;
        self.account_error = None;
        Task::perform(
            fetch_account_data_scoped_with_provider(addr, scope, provider, hydromancer_key),
            move |r| {
                Message::AccountDataLoaded(
                    requested_addr.clone().into(),
                    account_context,
                    Box::new(r),
                )
            },
        )
    }

    pub(crate) fn refresh_account_data_for_twap_reconciliation(
        &mut self,
        addr: String,
    ) -> Task<Message> {
        if self.connected_address.as_deref() == Some(addr.as_str()) {
            if self.account_loading {
                self.account_refresh_followup_pending = true;
                self.account_reconciliation_required = true;
                return Task::none();
            }
            return self.force_refresh_account_data_for_reconciliation(addr);
        }

        let provider = self.read_data_provider;
        let account_context = self.begin_twap_reconciliation_account_data_request_context(&addr);
        let hydromancer_key = self.hydromancer_api_key_for_task();
        let requested_addr = addr.clone();
        let scope = self.account_data_fetch_scope();
        Task::perform(
            fetch_account_data_scoped_with_provider(addr, scope, provider, hydromancer_key),
            move |r| {
                Message::AccountDataLoaded(
                    requested_addr.clone().into(),
                    account_context,
                    Box::new(r),
                )
            },
        )
    }

    pub(crate) fn retry_twap_reconciliation_account_data(
        &mut self,
        address: String,
    ) -> Task<Message> {
        if !self.twap_reconciliation_account_data_retry_needed(&address) {
            return Task::none();
        }
        self.refresh_account_data_for_twap_reconciliation(address)
    }
}

fn fills_incomplete_warning(data: &AccountData) -> Option<String> {
    (!data.completeness.fills_complete).then(|| {
        data.completeness
            .section_warning(AccountDataSection::Fills)
            .unwrap_or_else(|| {
                "Trade history may be incomplete: refresh account data before reconciling TWAP fills"
                    .to_string()
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary, Position,
        PositionLeverage, SpotClearinghouseState, UserFeeRates, UserFill,
    };
    use crate::read_data_provider::{AccountDataRequestContext, ReadDataRequestContext};

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

    fn user_fill(coin: &str, side: &str, time: u64) -> UserFill {
        UserFill {
            coin: coin.to_string(),
            px: "100".to_string(),
            sz: "0.1".to_string(),
            side: side.to_string(),
            time,
            hash: None,
            tid: None,
            oid: Some(time),
            dir: "Open Long".to_string(),
            closed_pnl: "0".to_string(),
            fee: "0.01".to_string(),
            fee_token: None,
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

        let context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            address,
            context,
            Ok(account_data_with_position("HYPE")),
        );

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
    fn account_refresh_stamps_stored_snapshot_owner() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());

        let context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            address.clone(),
            context,
            Ok(account_data_with_position("ETH")),
        );

        assert_eq!(
            terminal.account_data_address.as_deref(),
            Some(address.as_str())
        );
        assert!(terminal.connected_order_account_snapshot().is_some());
    }

    #[test]
    fn account_refresh_preserves_canonical_fill_symbols_and_wire_sides() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        let mut data = account_data_with_position("HYPE");
        data.fills = vec![
            user_fill("BTC", "B", 1),
            user_fill("flx:BTC", "B", 2),
            user_fill("@107", "A", 3),
            user_fill("#950", "A", 4),
        ];

        let context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(address, context, Ok(data));

        let data = terminal.account_data.as_ref().expect("account data");
        let parsed: Vec<(&str, &str)> = data
            .fills
            .iter()
            .map(|fill| (fill.coin.as_str(), fill.side.as_str()))
            .collect();
        assert_eq!(
            parsed,
            vec![("BTC", "B"), ("flx:BTC", "B"), ("@107", "A"), ("#950", "A")]
        );
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

        let context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            address.clone(),
            context,
            Ok(account_data_with_position("HYPE")),
        );

        // The queued follow-up fires as soon as the stale snapshot lands.
        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_loading);
    }

    #[test]
    fn queued_refresh_followup_runs_after_in_flight_fetch_failure() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = true;
        terminal.account_reconciliation_required = true;
        terminal.account_refresh_followup_pending = true;
        let context = terminal.current_account_data_request_context();

        let _task = terminal.apply_account_data_loaded(
            address,
            context,
            Err("temporary account refresh failure".to_string()),
        );

        assert!(terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);
        assert!(terminal.account_error.is_none());
    }

    #[test]
    fn account_load_error_redacts_account_error() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = true;
        terminal.account_reconciliation_required = true;
        let context = terminal.current_account_data_request_context();

        let _task = terminal.apply_account_data_loaded(
            address,
            context,
            Err("refresh failed: api_key=account-secret signature=sig-secret".to_string()),
        );

        let error = terminal.account_error.as_deref().expect("account error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(error.contains("signature=<redacted>"));
        assert!(!error.contains("account-secret"));
        assert!(!error.contains("sig-secret"));
    }

    #[test]
    fn queued_refresh_followup_result_clears_reconciliation_required() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = true;
        terminal.account_reconciliation_required = true;
        terminal.account_refresh_followup_pending = true;

        let stale_context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            address.clone(),
            stale_context,
            Ok(account_data_with_position("ETH")),
        );
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert!(!terminal.account_refresh_followup_pending);

        let followup_context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            address,
            followup_context,
            Ok(account_data_with_position("BTC")),
        );

        assert!(!terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(!terminal.account_reconciliation_required);
        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "BTC");
    }

    #[test]
    fn refresh_during_backoff_marks_reconciliation_required_without_loading() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address);
        terminal.account_loading = false;
        terminal.account_reconciliation_required = false;
        let due_ms = TradingTerminal::now_ms() + 60_000;
        terminal.account_refresh_backoff_until_ms = Some(due_ms);

        let _task = terminal.refresh_account_data();

        assert!(!terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert_eq!(terminal.account_refresh_retry_due_ms, Some(due_ms));
        assert!(
            terminal
                .account_error
                .as_deref()
                .is_some_and(|error| error.contains("rate limited"))
        );
    }

    #[test]
    fn rate_limited_account_load_marks_reconciliation_and_schedules_retry() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = true;
        terminal.account_reconciliation_required = false;
        let context = terminal.current_account_data_request_context();

        let _task = terminal.apply_account_data_loaded(
            address,
            context,
            Err("HTTP 429 Too Many Requests".to_string()),
        );

        assert!(!terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert!(terminal.account_refresh_backoff_until_ms.is_some());
        assert_eq!(
            terminal.account_refresh_retry_due_ms,
            terminal.account_refresh_backoff_until_ms
        );
        assert!(
            terminal
                .account_error
                .as_deref()
                .is_some_and(|error| error.contains("rate limited"))
        );
    }

    #[test]
    fn account_backoff_retry_wakeup_starts_refresh_after_due_time() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        let due_ms = TradingTerminal::now_ms();
        terminal.connected_address = Some(address);
        terminal.account_refresh_backoff_until_ms = Some(due_ms);
        terminal.account_refresh_retry_due_ms = Some(due_ms);
        terminal.account_reconciliation_required = true;

        let _task = terminal.handle_account_refresh_backoff_elapsed(due_ms);

        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert_eq!(terminal.account_refresh_retry_due_ms, None);
    }

    #[test]
    fn account_backoff_retry_wakeup_does_not_queue_followup_when_refresh_already_started() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        let due_ms = TradingTerminal::now_ms();
        terminal.connected_address = Some(address);
        terminal.account_loading = true;
        terminal.account_refresh_backoff_until_ms = Some(due_ms);
        terminal.account_refresh_retry_due_ms = Some(due_ms);
        terminal.account_reconciliation_required = true;

        let _task = terminal.handle_account_refresh_backoff_elapsed(due_ms);

        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert!(!terminal.account_refresh_followup_pending);
        assert_eq!(terminal.account_refresh_retry_due_ms, None);
    }

    #[test]
    fn stale_account_backoff_retry_wakeup_does_not_start_refresh() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        let old_due_ms = 100;
        let current_due_ms = 200;
        terminal.connected_address = Some(address);
        terminal.account_refresh_backoff_until_ms = Some(current_due_ms);
        terminal.account_refresh_retry_due_ms = Some(current_due_ms);
        terminal.account_reconciliation_required = true;

        let _task = terminal.handle_account_refresh_backoff_elapsed(old_due_ms);

        assert!(!terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert_eq!(terminal.account_refresh_retry_due_ms, Some(current_due_ms));
    }

    #[test]
    fn twap_reconciliation_requested_mid_connected_fetch_is_queued() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_loading = true;
        terminal.account_reconciliation_required = false;

        let _task = terminal.refresh_account_data_for_twap_reconciliation(address);

        assert!(terminal.account_loading);
        assert!(terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);
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

        let context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            address.clone(),
            context,
            Ok(account_data_with_position("HYPE")),
        );
        assert!(!terminal.account_loading);
    }

    #[test]
    fn stale_same_account_refresh_result_does_not_overwrite_newer_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());

        let stale_context = terminal.begin_account_data_request_context();
        let current_context = terminal.begin_account_data_request_context();
        terminal.account_loading = true;

        let _task = terminal.apply_account_data_loaded(
            address.clone(),
            current_context,
            Ok(account_data_with_position("BTC")),
        );
        assert!(!terminal.account_loading);

        let _task = terminal.apply_account_data_loaded(
            address,
            stale_context,
            Ok(account_data_with_position("ETH")),
        );

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "BTC");
        assert!(!terminal.account_loading);
    }

    #[test]
    fn stale_same_account_refresh_result_does_not_consume_current_loading_or_followup() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.account_data = Some(account_data_with_position("ETH"));

        let stale_context = terminal.begin_account_data_request_context();
        let current_context = terminal.begin_account_data_request_context();
        terminal.account_loading = true;
        terminal.account_refresh_followup_pending = true;
        terminal.account_reconciliation_required = true;

        let _task = terminal.apply_account_data_loaded(
            address.clone(),
            stale_context,
            Ok(account_data_with_position("SOL")),
        );

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "ETH");
        assert!(terminal.account_loading);
        assert!(terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);

        let _task = terminal.apply_account_data_loaded(
            address,
            current_context,
            Ok(account_data_with_position("BTC")),
        );

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "BTC");
        assert!(terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);
    }

    #[test]
    fn stale_hydromancer_account_context_runs_queued_current_refresh_without_applying_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.read_data_provider = crate::config::ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        terminal.account_loading = true;
        terminal.account_refresh_followup_pending = true;
        terminal.account_reconciliation_required = true;
        terminal.account_data = Some(account_data_with_position("ETH"));
        let stale_read_context = ReadDataRequestContext {
            provider: crate::config::ReadDataProvider::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 1,
        };
        let stale_context = AccountDataRequestContext::connected_snapshot(
            stale_read_context,
            terminal.account_data_request_generation,
        );

        let _task = terminal.apply_account_data_loaded(
            address,
            stale_context,
            Ok(account_data_with_position("BTC")),
        );

        assert!(terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);
        let positions = &terminal
            .account_data
            .as_ref()
            .expect("account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "ETH");
    }

    #[test]
    fn provider_change_queues_current_refresh_and_rejects_old_provider_account_result() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.read_data_provider = crate::config::ReadDataProvider::Hyperliquid;
        terminal.account_data = Some(account_data_with_position("ETH"));

        let _task = terminal.refresh_account_data();
        let old_provider_context = terminal.current_account_data_request_context();
        assert!(terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);

        let _task = terminal.update_preferences(Message::ReadDataProviderChanged(
            crate::config::ReadDataProvider::Hydromancer,
        ));

        assert!(terminal.account_loading);
        assert!(terminal.account_refresh_followup_pending);

        let _task = terminal.apply_account_data_loaded(
            address,
            old_provider_context,
            Ok(account_data_with_position("BTC")),
        );

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("existing account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "ETH");
        assert!(terminal.account_loading);
        assert!(!terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);
    }

    #[test]
    fn provider_round_trip_rejects_old_hyperliquid_account_result() {
        let mut terminal = TradingTerminal::boot().0;
        let address = "0xabc0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(address.clone());
        terminal.read_data_provider = crate::config::ReadDataProvider::Hyperliquid;
        terminal.account_data = Some(account_data_with_position("ETH"));

        let _task = terminal.refresh_account_data();
        let old_context = terminal.current_account_data_request_context();

        let _task = terminal.update_preferences(Message::ReadDataProviderChanged(
            crate::config::ReadDataProvider::Hydromancer,
        ));
        let _task = terminal.update_preferences(Message::ReadDataProviderChanged(
            crate::config::ReadDataProvider::Hyperliquid,
        ));

        let _task = terminal.apply_account_data_loaded(
            address,
            old_context,
            Ok(account_data_with_position("BTC")),
        );

        let positions = &terminal
            .account_data
            .as_ref()
            .expect("existing account data")
            .clearinghouse
            .asset_positions;
        assert_eq!(positions[0].position.coin, "ETH");
        assert!(terminal.account_loading);
    }

    #[test]
    fn stale_off_account_context_does_not_clear_connected_account_refresh() {
        let mut terminal = TradingTerminal::boot().0;
        let connected_address = "0xabc0000000000000000000000000000000000000".to_string();
        let other_address = "0xdef0000000000000000000000000000000000000".to_string();
        terminal.connected_address = Some(connected_address);
        terminal.read_data_provider = crate::config::ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        terminal.account_loading = true;
        terminal.account_refresh_followup_pending = true;
        terminal.account_reconciliation_required = true;
        let stale_read_context = ReadDataRequestContext {
            provider: crate::config::ReadDataProvider::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 1,
        };
        let stale_context = AccountDataRequestContext::connected_snapshot(
            stale_read_context,
            terminal.account_data_request_generation,
        );

        let _task = terminal.apply_account_data_loaded(
            other_address,
            stale_context,
            Ok(account_data_with_position("BTC")),
        );

        assert!(terminal.account_loading);
        assert!(terminal.account_refresh_followup_pending);
        assert!(terminal.account_reconciliation_required);
    }
}
