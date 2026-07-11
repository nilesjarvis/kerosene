use super::results::result_requires_account_refresh;
use crate::account::AccountDataFetchScope;
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::order_execution::{OrderLeverageSubmissionSnapshot, PendingLeverageUpdateContext};
use crate::signing::{ExchangeResponse, update_leverage};

use iced::Task;

const DEFAULT_ORDER_LEVERAGE: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderLeverageConstraints {
    max_leverage: u32,
    cross_allowed: bool,
}

impl TradingTerminal {
    pub(crate) fn handle_toggle_order_leverage_dropdown(&mut self) {
        if self.active_order_leverage_constraints().is_some() {
            self.order_leverage_dropdown_open = !self.order_leverage_dropdown_open;
        } else {
            self.order_leverage_dropdown_open = false;
        }
    }

    pub(crate) fn handle_order_leverage_input_changed(&mut self, value: String) {
        self.order_leverage_input = sanitize_leverage_input(&value);
    }

    pub(crate) fn handle_set_order_leverage_cross(&mut self, is_cross: bool) {
        if is_cross
            && self
                .active_order_leverage_constraints()
                .is_some_and(|(_, cross_allowed)| !cross_allowed)
        {
            self.order_leverage_is_cross = false;
            self.order_status = Some((
                format!(
                    "{} only supports isolated margin",
                    self.active_symbol_display.to_uppercase()
                ),
                true,
            ));
            return;
        }

        self.order_leverage_is_cross = is_cross;
    }

    pub(crate) fn order_leverage_submission_snapshot(&self) -> OrderLeverageSubmissionSnapshot {
        OrderLeverageSubmissionSnapshot {
            symbol_key: self.active_symbol.clone(),
            leverage_input: self.order_leverage_input.clone(),
            is_cross: self.order_leverage_is_cross,
        }
    }

    fn order_leverage_submission_snapshot_matches(
        &self,
        snapshot: &OrderLeverageSubmissionSnapshot,
    ) -> bool {
        self.active_symbol == snapshot.symbol_key
            && self.order_leverage_input == snapshot.leverage_input
            && self.order_leverage_is_cross == snapshot.is_cross
    }

    pub(crate) fn submit_order_leverage_update(
        &mut self,
        snapshot: OrderLeverageSubmissionSnapshot,
    ) -> Task<Message> {
        if self.pending_leverage_update.is_some() {
            return Task::none();
        }
        if !self.order_leverage_submission_snapshot_matches(&snapshot) {
            self.order_status = Some((
                "Leverage settings changed; review and apply again".into(),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_pending_trading_request("updating leverage") {
            return Task::none();
        }
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before updating leverage"
                    .into(),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("updating leverage", "account data") {
            return Task::none();
        }

        let Some((key, address)) = self.order_signing_context() else {
            return Task::none();
        };

        let Some(symbol) = self
            .resolve_exchange_symbol_by_key_or_ticker(&snapshot.symbol_key)
            .filter(|symbol| {
                symbol.market_type == MarketType::Perp && self.exchange_symbol_is_orderable(symbol)
            })
            .cloned()
        else {
            self.order_status = Some((
                "Leverage is only available for perpetual markets".into(),
                true,
            ));
            return Task::none();
        };

        let constraints = order_leverage_constraints_for_symbol(&symbol);
        let Some(leverage) = parse_leverage_input(&snapshot.leverage_input) else {
            self.order_status = Some(("Enter leverage as a whole number".into(), true));
            return Task::none();
        };
        if leverage > constraints.max_leverage {
            self.order_status = Some((
                format!(
                    "Max leverage for {} is {}x",
                    Self::exchange_symbol_display_name(&symbol).to_uppercase(),
                    constraints.max_leverage
                ),
                true,
            ));
            return Task::none();
        }

        let is_cross = snapshot.is_cross && constraints.cross_allowed;
        if snapshot.is_cross && !constraints.cross_allowed {
            self.order_leverage_is_cross = false;
            self.order_status = Some((
                format!(
                    "{} only supports isolated margin",
                    Self::exchange_symbol_display_name(&symbol).to_uppercase()
                ),
                true,
            ));
            return Task::none();
        }

        let context = PendingLeverageUpdateContext {
            address,
            symbol_key: symbol.key.clone(),
            display: Self::exchange_symbol_display_name(&symbol),
            asset: symbol.asset_index,
            dex: symbol
                .key
                .split_once(':')
                .map(|(dex, _)| dex.to_ascii_lowercase()),
            is_cross,
            leverage,
        };

        self.pending_leverage_update = Some(context.clone());
        self.order_status = Some(("Updating leverage...".into(), false));

        Task::perform(
            update_leverage(key, context.asset, context.is_cross, context.leverage),
            move |result| Message::OrderLeverageResult {
                context: context.clone(),
                result: result.into(),
            },
        )
    }

    pub(crate) fn handle_order_leverage_result(
        &mut self,
        context: PendingLeverageUpdateContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        if self.pending_leverage_update.as_ref() != Some(&context) {
            return Task::none();
        }
        self.pending_leverage_update = None;

        if !self.connected_order_account_matches(&context.address) {
            return Task::none();
        }

        let should_refresh = result_requires_account_refresh(&result);

        match result {
            Ok(response) if response.is_confirmed_default_result() => {
                self.order_leverage_dropdown_open = false;
                if self.active_symbol == context.symbol_key {
                    self.order_leverage_input = context.leverage.to_string();
                    self.order_leverage_is_cross = context.is_cross;
                }
                self.order_status = Some((
                    format!(
                        "{} leverage updated: {} {}x",
                        context.display.to_uppercase(),
                        context.margin_mode_label(),
                        context.leverage
                    ),
                    false,
                ));
            }
            Ok(response) if !response.is_error() => {
                let summary = redact_sensitive_response_text(&response.summary());
                self.order_status = Some((
                    format!(
                        "Leverage update status uncertain: {}; refreshing account data",
                        summary
                    ),
                    true,
                ));
            }
            Ok(response) => {
                self.order_status =
                    Some((redact_sensitive_response_text(&response.summary()), true));
            }
            Err(error) => {
                self.order_status = Some((redact_sensitive_response_text(&error), true));
            }
        }

        if should_refresh {
            let scope = context
                .dex
                .as_deref()
                .map(AccountDataFetchScope::hip3_dex)
                .unwrap_or_else(|| self.account_data_fetch_scope());
            self.force_refresh_account_data_with_scope(context.address, scope)
        } else {
            Task::none()
        }
    }

    pub(crate) fn sync_order_leverage_form_for_active_symbol(&mut self) {
        let Some(symbol) = self.active_order_leverage_symbol() else {
            self.order_leverage_input = DEFAULT_ORDER_LEVERAGE.to_string();
            self.order_leverage_is_cross = false;
            self.order_leverage_dropdown_open = false;
            return;
        };

        let symbol_key = symbol.key.clone();
        let constraints = order_leverage_constraints_for_symbol(symbol);
        let account_setting = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data)
            .and_then(|data| data.get_leverage_for(&symbol_key, &self.exchange_symbols))
            .filter(|(_, _, is_actual)| *is_actual);
        let existing = parse_leverage_input(&self.order_leverage_input)
            .unwrap_or(DEFAULT_ORDER_LEVERAGE)
            .clamp(DEFAULT_ORDER_LEVERAGE, constraints.max_leverage);
        let leverage = account_setting
            .map(|(_, leverage, _)| leverage)
            .unwrap_or(existing)
            .clamp(DEFAULT_ORDER_LEVERAGE, constraints.max_leverage);
        let is_cross = account_setting
            .map(|(is_cross, _, _)| is_cross)
            .unwrap_or(constraints.cross_allowed)
            && constraints.cross_allowed;

        self.order_leverage_input = leverage.to_string();
        self.order_leverage_is_cross = is_cross;
    }

    pub(crate) fn active_order_leverage_constraints(&self) -> Option<(u32, bool)> {
        self.active_order_leverage_symbol()
            .map(order_leverage_constraints_for_symbol)
            .map(|constraints| (constraints.max_leverage, constraints.cross_allowed))
    }

    fn active_order_leverage_symbol(&self) -> Option<&ExchangeSymbol> {
        let symbol = self.resolve_exchange_symbol_by_key_or_ticker(&self.active_symbol)?;
        (symbol.market_type == MarketType::Perp && self.exchange_symbol_is_orderable(symbol))
            .then_some(symbol)
    }
}

fn order_leverage_constraints_for_symbol(symbol: &ExchangeSymbol) -> OrderLeverageConstraints {
    OrderLeverageConstraints {
        max_leverage: symbol.max_leverage.max(DEFAULT_ORDER_LEVERAGE),
        cross_allowed: !symbol.only_isolated,
    }
}

fn sanitize_leverage_input(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .take(3)
        .collect()
}

fn parse_leverage_input(value: &str) -> Option<u32> {
    let leverage = value.trim().parse::<u32>().ok()?;
    (leverage >= DEFAULT_ORDER_LEVERAGE).then_some(leverage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
        Position, PositionLeverage, SpotClearinghouseState,
    };
    use crate::api::MarketType;
    use crate::app_state::sensitive_string;
    use crate::config::AccountProfile;
    use crate::order_execution::{PendingNukeExecution, PendingOrderAction};

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

    fn symbol(key: &str, max_leverage: u32, only_isolated: bool) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.split(':').nth(1).unwrap_or(key).to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage,
            only_isolated,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    #[test]
    fn leverage_input_sanitizer_keeps_digits_only() {
        assert_eq!(sanitize_leverage_input(" 12x "), "12");
        assert_eq!(sanitize_leverage_input("abc"), "");
        assert_eq!(sanitize_leverage_input("1234"), "123");
    }

    #[test]
    fn leverage_input_parser_requires_positive_integer() {
        assert_eq!(parse_leverage_input("1"), Some(1));
        assert_eq!(parse_leverage_input("50"), Some(50));
        assert_eq!(parse_leverage_input("0"), None);
        assert_eq!(parse_leverage_input("1.5"), None);
    }

    #[test]
    fn isolated_only_symbol_disallows_cross() {
        let constraints = order_leverage_constraints_for_symbol(&symbol("xyz:NVDA", 10, true));

        assert_eq!(
            constraints,
            OrderLeverageConstraints {
                max_leverage: 10,
                cross_allowed: false,
            }
        );
    }

    #[test]
    fn leverage_constraints_never_expose_zero_max() {
        let constraints = order_leverage_constraints_for_symbol(&symbol("BTC", 0, false));

        assert_eq!(constraints.max_leverage, 1);
        assert!(constraints.cross_allowed);
    }

    #[test]
    fn leverage_dropdown_toggles_only_for_active_leverage_symbol() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", 50, false)];

        terminal.handle_toggle_order_leverage_dropdown();
        assert!(terminal.order_leverage_dropdown_open);

        terminal.handle_toggle_order_leverage_dropdown();
        assert!(!terminal.order_leverage_dropdown_open);

        terminal.active_symbol = "ETH".to_string();
        terminal.handle_toggle_order_leverage_dropdown();
        assert!(!terminal.order_leverage_dropdown_open);
    }

    fn pending_context(
        symbol_key: &str,
        is_cross: bool,
        leverage: u32,
    ) -> PendingLeverageUpdateContext {
        PendingLeverageUpdateContext {
            address: "0xabc".to_string(),
            symbol_key: symbol_key.to_string(),
            display: symbol_key.to_string(),
            asset: 0,
            dex: symbol_key
                .split_once(':')
                .map(|(dex, _)| dex.to_ascii_lowercase()),
            is_cross,
            leverage,
        }
    }

    fn leverage_submit_terminal() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.wallet_address_input = TEST_ACCOUNT.to_string();
        terminal.accounts = vec![AccountProfile {
            secret_id: "acct-a".to_string(),
            name: "Account A".to_string(),
            wallet_address: TEST_ACCOUNT.to_string(),
            agent_key: sensitive_string("").into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }];
        terminal.active_account_index = 0;
        terminal.set_committed_agent_key_for_test("agent-key");
        terminal.active_symbol = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", 50, false)];
        terminal.order_leverage_input = "5".to_string();
        terminal
    }

    fn account_data_with_leverage(coin: &str, is_cross: bool, leverage: u32) -> AccountData {
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
                            leverage_type: if is_cross { "cross" } else { "isolated" }.to_string(),
                            value: leverage,
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
            fee_rates: Default::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    fn ok_exchange_response() -> ExchangeResponse {
        serde_json::from_str(r#"{"status":"ok","response":{"type":"default"}}"#)
            .expect("valid exchange response")
    }

    fn raw_ok_exchange_response() -> ExchangeResponse {
        serde_json::from_str(r#"{"status":"ok","response":"schema-shifted"}"#)
            .expect("valid exchange response")
    }

    fn missing_body_ok_exchange_response() -> ExchangeResponse {
        serde_json::from_str(r#"{"status":"ok"}"#).expect("valid exchange response")
    }

    fn order_shaped_ok_exchange_response() -> ExchangeResponse {
        serde_json::from_str(
            r#"{"status":"ok","response":{"type":"order","data":{"statuses":[{"resting":{"oid":42}}]}}}"#,
        )
        .expect("valid exchange response")
    }

    fn error_exchange_response() -> ExchangeResponse {
        serde_json::from_str(
            r#"{"status":"ok","response":{"type":"default","data":{"statuses":[{"error":"update rejected api_key=super-secret"}]}}}"#,
        )
        .expect("valid exchange response")
    }

    fn assert_stale_leverage_snapshot_rejected(terminal: &TradingTerminal) {
        assert!(terminal.pending_leverage_update.is_none());
        assert_eq!(
            terminal.order_status,
            Some((
                "Leverage settings changed; review and apply again".to_string(),
                true
            ))
        );
    }

    fn assert_uncertain_leverage_result_refreshes(
        terminal: &TradingTerminal,
        expected_summary: &str,
    ) {
        assert_eq!(terminal.pending_leverage_update, None);
        assert!(terminal.order_leverage_dropdown_open);
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);

        let Some((message, is_error)) = &terminal.order_status else {
            panic!("missing leverage status");
        };
        assert!(*is_error);
        assert!(message.contains("Leverage update status uncertain"));
        assert!(message.contains(expected_summary));
        assert!(message.contains("refreshing account data"));
    }

    #[test]
    fn leverage_submit_rejects_pending_order_action() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.pending_order_action = Some(PendingOrderAction::Buy);

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert!(terminal.pending_leverage_update.is_none());
        assert_eq!(
            terminal.order_status,
            Some((
                "Wait for pending trading requests to finish before updating leverage".to_string(),
                true
            ))
        );
    }

    #[test]
    fn leverage_submit_rejects_pending_nuke_execution() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.pending_nuke_execution = Some(PendingNukeExecution::new(1, 1, 0));

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert!(terminal.pending_leverage_update.is_none());
        assert_eq!(
            terminal.order_status,
            Some((
                "Wait for pending trading requests to finish before updating leverage".to_string(),
                true
            ))
        );
    }

    #[test]
    fn leverage_submit_rejects_pending_account_reconciliation() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.account_reconciliation_required = true;

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert!(terminal.pending_leverage_update.is_none());
        assert_eq!(
            terminal.order_status,
            Some((
                "Account refresh pending; wait for fresh account data before updating leverage"
                    .to_string(),
                true
            ))
        );
    }

    #[test]
    fn leverage_submit_rejects_account_loading_even_without_reconciliation_flag() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.account_loading = true;
        terminal.account_reconciliation_required = false;

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert!(terminal.pending_leverage_update.is_none());
        assert_eq!(
            terminal.order_status,
            Some((
                "Account refresh in progress; wait for fresh account data before updating leverage"
                    .to_string(),
                true
            ))
        );
    }

    #[test]
    fn leverage_submit_rejects_stale_symbol_snapshot() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.active_symbol = "ETH".to_string();

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert_stale_leverage_snapshot_rejected(&terminal);
    }

    #[test]
    fn leverage_submit_rejects_changed_input_snapshot() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.order_leverage_input = "6".to_string();

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert_stale_leverage_snapshot_rejected(&terminal);
    }

    #[test]
    fn leverage_submit_rejects_changed_margin_mode_snapshot() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        terminal.order_leverage_is_cross = !terminal.order_leverage_is_cross;

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert_stale_leverage_snapshot_rejected(&terminal);
    }

    #[test]
    fn leverage_submit_ignores_duplicate_while_pending_even_if_snapshot_is_stale() {
        let mut terminal = leverage_submit_terminal();
        let snapshot = terminal.order_leverage_submission_snapshot();
        let pending = pending_context("BTC", true, 5);
        terminal.pending_leverage_update = Some(pending.clone());
        terminal.order_status = Some(("Updating leverage...".to_string(), false));
        terminal.order_leverage_input = "6".to_string();

        let _task = terminal.submit_order_leverage_update(snapshot);

        assert_eq!(terminal.pending_leverage_update, Some(pending));
        assert_eq!(
            terminal.order_status,
            Some(("Updating leverage...".to_string(), false))
        );
    }

    #[test]
    fn leverage_form_sync_ignores_stale_account_snapshot() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());
        terminal.account_data = Some(account_data_with_leverage("BTC", false, 25));
        terminal.active_symbol = "BTC".to_string();
        terminal.exchange_symbols = vec![symbol("BTC", 50, false)];
        terminal.order_leverage_input = "7".to_string();
        terminal.order_leverage_is_cross = false;

        terminal.sync_order_leverage_form_for_active_symbol();

        assert_eq!(terminal.order_leverage_input, "7");
        assert!(terminal.order_leverage_is_cross);
    }

    #[test]
    fn leverage_result_uses_submitted_context_not_current_form() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.active_symbol = "BTC".to_string();
        terminal.order_leverage_input = "99".to_string();
        terminal.order_leverage_is_cross = false;
        terminal.order_leverage_dropdown_open = true;
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ = terminal.handle_order_leverage_result(context, Ok(ok_exchange_response()));

        assert_eq!(terminal.pending_leverage_update, None);
        assert!(!terminal.order_leverage_dropdown_open);
        assert_eq!(terminal.order_leverage_input, "12");
        assert!(terminal.order_leverage_is_cross);
        assert_eq!(
            terminal.order_status,
            Some(("BTC leverage updated: Cross 12x".to_string(), false))
        );
    }

    #[test]
    fn leverage_result_raw_ok_does_not_confirm_success_and_refreshes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.active_symbol = "BTC".to_string();
        terminal.order_leverage_input = "99".to_string();
        terminal.order_leverage_is_cross = false;
        terminal.order_leverage_dropdown_open = true;
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ = terminal.handle_order_leverage_result(context, Ok(raw_ok_exchange_response()));

        assert_eq!(terminal.order_leverage_input, "99");
        assert!(!terminal.order_leverage_is_cross);
        assert_uncertain_leverage_result_refreshes(&terminal, "No response body");
    }

    #[test]
    fn leverage_result_missing_body_ok_does_not_confirm_success_and_refreshes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.active_symbol = "BTC".to_string();
        terminal.order_leverage_input = "99".to_string();
        terminal.order_leverage_is_cross = false;
        terminal.order_leverage_dropdown_open = true;
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ =
            terminal.handle_order_leverage_result(context, Ok(missing_body_ok_exchange_response()));

        assert_eq!(terminal.order_leverage_input, "99");
        assert!(!terminal.order_leverage_is_cross);
        assert_uncertain_leverage_result_refreshes(&terminal, "No response body");
    }

    #[test]
    fn leverage_result_order_shaped_ok_does_not_confirm_success_and_refreshes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.active_symbol = "BTC".to_string();
        terminal.order_leverage_input = "99".to_string();
        terminal.order_leverage_is_cross = false;
        terminal.order_leverage_dropdown_open = true;
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ =
            terminal.handle_order_leverage_result(context, Ok(order_shaped_ok_exchange_response()));

        assert_eq!(terminal.order_leverage_input, "99");
        assert!(!terminal.order_leverage_is_cross);
        assert_uncertain_leverage_result_refreshes(&terminal, "Resting (oid 42)");
    }

    #[test]
    fn leverage_result_exchange_error_redacts_sensitive_status_text() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ = terminal.handle_order_leverage_result(context, Ok(error_exchange_response()));

        assert_eq!(terminal.pending_leverage_update, None);
        assert!(!terminal.account_loading);
        let (message, is_error) = terminal.order_status.expect("status should be set");
        assert!(is_error);
        assert!(message.contains("update rejected"));
        assert!(message.contains("api_key=<redacted>"));
        assert!(!message.contains("super-secret"));
    }

    #[test]
    fn leverage_result_transport_error_redacts_sensitive_status_text() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ = terminal.handle_order_leverage_result(
            context,
            Err("leverage request failed: private_key=super-secret".to_string()),
        );

        assert_eq!(terminal.pending_leverage_update, None);
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        let (message, is_error) = terminal.order_status.expect("status should be set");
        assert!(is_error);
        assert!(message.contains("leverage request failed"));
        assert!(message.contains("private_key=<redacted>"));
        assert!(!message.contains("super-secret"));
    }

    #[test]
    fn leverage_result_ignores_stale_context() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        let current = pending_context("ETH", false, 3);
        terminal.pending_leverage_update = Some(current.clone());

        let _ = terminal.handle_order_leverage_result(
            pending_context("BTC", true, 12),
            Ok(ok_exchange_response()),
        );

        assert_eq!(terminal.pending_leverage_update, Some(current));
        assert_eq!(terminal.order_status, None);
    }
}
