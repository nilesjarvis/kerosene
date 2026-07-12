use crate::api::{OrderStatusResult, fetch_order_status_by_oid};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::order_execution::MoveOrderKey;
use crate::signing::ExchangeResponse;
use iced::Task;

use super::results::{ExecutionOutcomeKind, PendingMoveStatusRequest, classify_execution_result};

impl TradingTerminal {
    fn move_order_status_task(account_address: String, coin: String, oid: u64) -> Task<Message> {
        Task::perform(
            fetch_order_status_by_oid(account_address.clone(), oid),
            move |result| Message::MoveOrderStatusLoaded {
                account_address: account_address.into(),
                coin,
                oid,
                result: Box::new(result),
            },
        )
    }

    pub(super) fn handle_move_order_modify_result(
        &mut self,
        account_address: String,
        coin: String,
        oid: u64,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let move_key = MoveOrderKey::new(coin, oid);
        let Some(pending_context) = self.pending_move_order_contexts.get(&move_key) else {
            self.sync_all_chart_orders();
            return Task::none();
        };
        let pending_context_matches_result_account =
            pending_context.matches_account(&account_address);
        if !self.connected_order_account_matches(&account_address) {
            if pending_context_matches_result_account {
                self.pending_move_order_contexts.remove(&move_key);
                self.clear_pending_order_indicator(pending_indicator_id);
            }
            self.sync_all_chart_orders();
            return Task::none();
        }
        if !pending_context_matches_result_account {
            self.sync_all_chart_orders();
            return Task::none();
        }

        let confirmed_price = self.pending_modification_price(pending_indicator_id);
        let response_oid = result.as_ref().ok().and_then(|resp| resp.order_oid());
        self.pending_move_order_contexts.remove(&move_key);
        self.clear_pending_order_indicator(pending_indicator_id);

        let mut outcome = classify_execution_result(result);
        // Carry the confirmed price into the local snapshot so the order line
        // does not snap back to the old price between the modify ack and the
        // next authoritative open-orders update.
        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::AcceptedResting | ExecutionOutcomeKind::Filled
        ) && let Some(price) = confirmed_price
            && let Some(order) = self
                .account_data_for_order_account_mut(&account_address)
                .and_then(|data| {
                    data.open_orders
                        .iter_mut()
                        .find(|order| order.oid == oid && order.coin == move_key.coin())
                })
        {
            order.limit_px = price;
            // Hyperliquid modifies have kept the oid stable so far, but adopt
            // the oid echoed in the response in case a modify ever re-keys the
            // order (parity with the chase modify handler) — a follow-up
            // cancel or move must target the live order, not a dead oid.
            if let Some(response_oid) = response_oid {
                order.oid = response_oid;
            }
        }
        self.sync_all_chart_orders();
        match outcome.kind {
            ExecutionOutcomeKind::Rejected => {
                outcome.status = format!("Move failed: {}", outcome.status);
            }
            ExecutionOutcomeKind::Ambiguous => {
                self.pending_move_status_request = Some(PendingMoveStatusRequest::new(
                    account_address.clone(),
                    oid,
                    move_key.coin().to_string(),
                ));
                self.set_order_status(
                    format!(
                        "Move modify status unknown for order {oid}: {}; checking orderStatus and refreshing account data",
                        outcome.status
                    ),
                    true,
                );
                return Task::batch([
                    self.refresh_account_data(),
                    Self::move_order_status_task(account_address, move_key.coin().to_string(), oid),
                ]);
            }
            ExecutionOutcomeKind::TransportUnknown => {
                self.pending_move_status_request = Some(PendingMoveStatusRequest::new(
                    account_address.clone(),
                    oid,
                    move_key.coin().to_string(),
                ));
                self.set_order_status(
                    format!(
                        "Move modify status unknown for order {oid}: {}; checking orderStatus and refreshing account data",
                        outcome.status
                    ),
                    true,
                );
                return Task::batch([
                    self.refresh_account_data(),
                    Self::move_order_status_task(account_address, move_key.coin().to_string(), oid),
                ]);
            }
            ExecutionOutcomeKind::AcceptedResting
            | ExecutionOutcomeKind::Filled
            | ExecutionOutcomeKind::Cancelled => {}
        }
        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn handle_move_order_status_result(
        &mut self,
        account_address: String,
        coin: String,
        oid: u64,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let request_matches = self
            .pending_move_status_request
            .as_ref()
            .is_some_and(|pending| pending.matches(&account_address, oid, &coin));
        if !request_matches {
            self.sync_all_chart_orders();
            return Task::none();
        }

        if !self.connected_order_account_matches(&account_address) {
            self.pending_move_status_request = None;
            self.sync_all_chart_orders();
            return Task::none();
        }

        match result {
            Ok(status) if status.is_open() => {
                self.set_order_status(
                    format!(
                        "Move modify status still uncertain for order {oid}: orderStatus reports open ({}); refreshing account data to confirm price",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_filled() => {
                self.pending_move_status_request = None;
                self.set_order_status(
                    format!(
                        "Move modify resolved by fill for order {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_no_fill_terminal() => {
                self.pending_move_status_request = None;
                self.set_order_status(
                    format!(
                        "Move modify resolved without an open order for {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_missing() => {
                self.set_order_status(
                    format!(
                        "Move modify status still uncertain for order {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) => {
                self.set_order_status(
                    format!(
                        "Move modify status still uncertain for order {oid}: orderStatus returned {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Err(error) => {
                let error = redact_sensitive_response_text(&error);
                self.set_order_status(
                    format!(
                        "Move modify status still uncertain for order {oid}: {error}; refreshing account data"
                    ),
                    true,
                );
            }
        }

        self.refresh_account_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::order_execution::{MoveOrderKey, PendingMoveOrderContext};
    use crate::timeframe::Timeframe;
    use zeroize::Zeroizing;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

    fn open_order(oid: u64, limit_px: &str) -> crate::account::OpenOrder {
        open_order_for("BTC", oid, limit_px)
    }

    fn open_order_for(coin: &str, oid: u64, limit_px: &str) -> crate::account::OpenOrder {
        crate::account::OpenOrder {
            coin: coin.to_string(),
            side: "B".to_string(),
            limit_px: limit_px.to_string(),
            sz: "1".to_string(),
            oid,
            timestamp: 1,
            reduce_only: Some(false),
            is_trigger: None,
            order_type: None,
            tif: None,
            trigger_px: None,
        }
    }

    fn account_data_with_order(order: crate::account::OpenOrder) -> crate::account::AccountData {
        account_data_with_orders(vec![order])
    }

    fn account_data_with_orders(
        orders: Vec<crate::account::OpenOrder>,
    ) -> crate::account::AccountData {
        crate::account::AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: crate::account::ClearinghouseState {
                margin_summary: crate::account::MarginSummary {
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
            spot: crate::account::SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: orders,
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: crate::account::UserFeeRates::default(),
            completeness: crate::account::AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    fn resting_response() -> ExchangeResponse {
        resting_response_with_oid(42)
    }

    fn resting_response_with_oid(oid: u64) -> ExchangeResponse {
        serde_json::from_value(serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{ "resting": { "oid": oid } }]
                }
            }
        }))
        .expect("test exchange response should deserialize")
    }

    fn malformed_ok_response() -> ExchangeResponse {
        serde_json::from_value(serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": "schema-shifted"
                }
            }
        }))
        .expect("test exchange response should deserialize")
    }

    fn order_status(status: &str) -> OrderStatusResult {
        OrderStatusResult {
            status: status.to_string(),
            oid: Some(42),
            cloid: None,
            raw_summary: format!("{status} (oid 42)"),
        }
    }

    fn arm_pending_move_status_request(
        terminal: &mut TradingTerminal,
        account_address: &str,
        oid: u64,
        symbol: &str,
    ) {
        // The status request is armed only after the original modify result has
        // consumed its indicator and context.
        terminal.pending_order_indicators.clear();
        terminal.pending_move_order_contexts.clear();
        terminal.pending_move_status_request = Some(PendingMoveStatusRequest::new(
            account_address.to_string(),
            oid,
            symbol.to_string(),
        ));
    }

    fn finish_current_account_refresh(terminal: &mut TradingTerminal) {
        let context = terminal.current_account_data_request_context();
        let _task = terminal.apply_account_data_loaded(
            TEST_ACCOUNT.to_string(),
            context,
            Ok(account_data_with_orders(Vec::new())),
        );
    }

    fn terminal_with_pending_move() -> (TradingTerminal, Option<u64>) {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        let order = open_order(42, "100");
        terminal.set_account_data_for_address_for_test(
            TEST_ACCOUNT,
            account_data_with_order(order.clone()),
        );
        let pending_id = terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &order,
            "111".to_string(),
        );
        terminal.pending_move_order_contexts.insert(
            MoveOrderKey::new("BTC", 42),
            PendingMoveOrderContext::new(
                TEST_ACCOUNT.to_string(),
                Zeroizing::new("agent-key".to_string()),
            )
            .expect("pending move context"),
        );
        assert!(pending_id.is_some());
        (terminal, pending_id)
    }

    #[test]
    fn move_result_success_carries_confirmed_price_into_local_snapshot() {
        let (mut terminal, pending_id) = terminal_with_pending_move();

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(resting_response()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(terminal.pending_move_status_request.is_none());
        assert!(!terminal.has_pending_trading_request());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "111");
        let chart = &terminal.charts.get(&1).expect("chart").chart;
        assert_eq!(chart.active_orders.len(), 1);
        assert_eq!(chart.active_orders[0].limit_px, 111.0);
    }

    #[test]
    fn move_result_success_patches_only_matching_symbol_for_same_oid() {
        let (mut terminal, _old_pending_id) = terminal_with_pending_move();
        let btc_order = open_order_for("BTC", 42, "100");
        let eth_order = open_order_for("ETH", 42, "200");
        terminal.set_account_data_for_address_for_test(
            TEST_ACCOUNT,
            account_data_with_orders(vec![btc_order, eth_order.clone()]),
        );
        terminal.pending_order_indicators.clear();
        terminal.pending_move_order_contexts.clear();
        let pending_id = terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &eth_order,
            "211".to_string(),
        );
        terminal.pending_move_order_contexts.insert(
            MoveOrderKey::new("ETH", 42),
            PendingMoveOrderContext::new(
                TEST_ACCOUNT.to_string(),
                Zeroizing::new("agent-key".to_string()),
            )
            .expect("pending move context"),
        );

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "ETH".to_string(),
            42,
            pending_id,
            Ok(resting_response()),
        );

        let data = terminal.account_data.as_ref().expect("account data");
        let btc = data
            .open_orders
            .iter()
            .find(|order| order.coin == "BTC")
            .expect("btc order");
        let eth = data
            .open_orders
            .iter()
            .find(|order| order.coin == "ETH")
            .expect("eth order");
        assert_eq!(btc.limit_px, "100");
        assert_eq!(eth.limit_px, "211");
    }

    #[test]
    fn move_result_success_adopts_resting_oid_from_response() {
        let (mut terminal, pending_id) = terminal_with_pending_move();

        // If the exchange ever re-keys an order on modify, follow-up cancels
        // and moves must target the oid from the response, not a dead one
        // (parity with the chase modify handler).
        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(resting_response_with_oid(77)),
        );

        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].oid, 77);
        assert_eq!(data.open_orders[0].limit_px, "111");
    }

    #[test]
    fn move_result_without_pending_context_is_ignored() {
        let (mut terminal, pending_id) = terminal_with_pending_move();
        terminal.pending_move_order_contexts.clear();
        terminal.order_status = None;

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(resting_response_with_oid(77)),
        );

        assert!(terminal.order_status.is_none());
        assert!(
            terminal
                .pending_order_indicators
                .contains_key(&pending_id.unwrap())
        );
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].oid, 42);
        assert_eq!(data.open_orders[0].limit_px, "100");
    }

    #[test]
    fn move_result_success_ignores_order_from_stale_account_snapshot() {
        let (mut terminal, pending_id) = terminal_with_pending_move();
        terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(resting_response_with_oid(77)),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(terminal.pending_move_status_request.is_none());
        assert!(!terminal.has_pending_trading_request());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].oid, 42);
        assert_eq!(data.open_orders[0].limit_px, "100");
    }

    #[test]
    fn move_result_failure_keeps_local_price() {
        let (mut terminal, pending_id) = terminal_with_pending_move();

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Err("exchange request failed: token=super-secret".to_string()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(terminal.pending_move_status_request.is_some());
        assert!(terminal.has_pending_trading_request());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(is_error);
        assert!(message.starts_with("Move modify status unknown"));
        assert!(message.contains("token=<redacted>"));
        assert!(!message.contains("super-secret"));

        finish_current_account_refresh(&mut terminal);

        assert!(terminal.pending_move_status_request.is_none());
        assert!(!terminal.has_pending_trading_request());
    }

    #[test]
    fn move_result_ambiguous_ack_is_uncertain_and_keeps_local_price() {
        let (mut terminal, pending_id) = terminal_with_pending_move();

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(malformed_ok_response()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(terminal.pending_move_status_request.is_some());
        assert!(terminal.has_pending_trading_request());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(is_error);
        assert!(message.contains("Move modify status unknown"));
        assert!(message.contains("refreshing account data"));

        finish_current_account_refresh(&mut terminal);

        assert!(terminal.pending_move_status_request.is_none());
        assert!(!terminal.has_pending_trading_request());
    }

    #[test]
    fn move_order_status_open_keeps_modify_uncertain() {
        let (mut terminal, _pending_id) = terminal_with_pending_move();
        arm_pending_move_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");

        let _task = terminal.handle_move_order_status_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            Ok(order_status("open")),
        );

        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
        assert!(terminal.pending_move_status_request.is_some());
        assert!(terminal.has_pending_trading_request());
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(is_error);
        assert!(message.contains("still uncertain"));
        assert!(message.contains("reports open"));

        finish_current_account_refresh(&mut terminal);

        assert!(terminal.pending_move_status_request.is_none());
        assert!(!terminal.has_pending_trading_request());
    }

    #[test]
    fn move_order_status_error_redacts_sensitive_text() {
        let (mut terminal, _pending_id) = terminal_with_pending_move();
        arm_pending_move_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");

        let _task = terminal.handle_move_order_status_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            Err("orderStatus request failed: api_key=super-secret".to_string()),
        );

        assert!(terminal.pending_move_status_request.is_some());
        assert!(terminal.has_pending_trading_request());
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(is_error);
        assert!(message.contains("Move modify status still uncertain"));
        assert!(message.contains("api_key=<redacted>"));
        assert!(!message.contains("super-secret"));

        finish_current_account_refresh(&mut terminal);

        assert!(terminal.pending_move_status_request.is_none());
        assert!(!terminal.has_pending_trading_request());
    }

    #[test]
    fn move_order_status_without_matching_pending_request_is_ignored() {
        let (mut terminal, _pending_id) = terminal_with_pending_move();
        arm_pending_move_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");

        let _task = terminal.handle_move_order_status_result(
            TEST_ACCOUNT.to_string(),
            "ETH".to_string(),
            42,
            Ok(order_status("filled")),
        );

        assert!(terminal.pending_move_status_request.is_some());
        assert!(terminal.order_status.is_none());
        assert!(!terminal.account_loading);
    }

    #[test]
    fn move_order_status_terminal_clears_pending_request() {
        let (mut terminal, _pending_id) = terminal_with_pending_move();
        arm_pending_move_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");

        let _task = terminal.handle_move_order_status_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            Ok(order_status("filled")),
        );

        assert!(terminal.pending_move_status_request.is_none());
        assert!(terminal.account_loading);
        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(!is_error);
        assert!(message.contains("Move modify resolved by fill"));
    }

    #[test]
    fn move_order_status_after_account_switch_skips_status() {
        let (mut terminal, _pending_id) = terminal_with_pending_move();
        arm_pending_move_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");
        terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
        terminal.order_status = None;

        let _task = terminal.handle_move_order_status_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            Ok(order_status("open")),
        );

        assert!(terminal.order_status.is_none());
        assert!(terminal.pending_move_status_request.is_none());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
    }

    #[test]
    fn move_result_after_account_switch_skips_status_and_price_update() {
        let (mut terminal, pending_id) = terminal_with_pending_move();
        terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
        terminal.order_status = None;

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(resting_response()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.pending_move_order_contexts.is_empty());
        assert!(terminal.order_status.is_none());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
    }

    #[test]
    fn stale_move_result_preserves_new_same_oid_pending_move_after_account_change() {
        let (mut terminal, pending_id) = terminal_with_pending_move();
        terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
        terminal.pending_move_order_contexts.insert(
            MoveOrderKey::new("BTC", 42),
            PendingMoveOrderContext::new(
                OTHER_ACCOUNT.to_string(),
                Zeroizing::new("other-agent-key".to_string()),
            )
            .expect("other pending move context"),
        );
        terminal.order_status = None;

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            "BTC".to_string(),
            42,
            pending_id,
            Ok(resting_response()),
        );

        assert!(
            terminal
                .pending_move_order_contexts
                .contains_key(&MoveOrderKey::new("BTC", 42))
        );
        assert!(!terminal.pending_order_indicators.is_empty());
        assert!(terminal.order_status.is_none());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
    }
}
