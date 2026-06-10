use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use iced::Task;

use super::results::{ExecutionOutcomeKind, classify_execution_result};

impl TradingTerminal {
    pub(super) fn handle_move_order_modify_result(
        &mut self,
        account_address: String,
        oid: u64,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let confirmed_price = self.pending_modification_price(pending_indicator_id);
        self.pending_move_order_contexts.remove(&oid);
        self.clear_pending_order_indicator(pending_indicator_id);
        if self.connected_address.as_deref() != Some(account_address.as_str()) {
            self.sync_all_chart_orders();
            return Task::none();
        }

        let mut outcome = classify_execution_result(result);
        // Carry the confirmed price into the local snapshot so the order line
        // does not snap back to the old price between the modify ack and the
        // next authoritative open-orders update.
        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::AcceptedResting | ExecutionOutcomeKind::Filled
        ) && let Some(price) = confirmed_price
            && let Some(order) = self
                .account_data
                .as_mut()
                .and_then(|data| data.open_orders.iter_mut().find(|order| order.oid == oid))
        {
            order.limit_px = price;
        }
        self.sync_all_chart_orders();
        match outcome.kind {
            ExecutionOutcomeKind::Rejected => {
                outcome.status = format!("Move failed: {}", outcome.status);
            }
            ExecutionOutcomeKind::TransportUnknown => {
                outcome.status = format!("Move modify failed: {}", outcome.status);
            }
            ExecutionOutcomeKind::AcceptedResting
            | ExecutionOutcomeKind::Filled
            | ExecutionOutcomeKind::Cancelled
            | ExecutionOutcomeKind::Ambiguous => {}
        }
        self.apply_execution_outcome(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
    const OTHER_ACCOUNT: &str = "0xdef0000000000000000000000000000000000000";

    fn open_order(oid: u64, limit_px: &str) -> crate::account::OpenOrder {
        crate::account::OpenOrder {
            coin: "BTC".to_string(),
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
            open_orders: vec![order],
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: crate::account::UserFeeRates::default(),
            completeness: crate::account::AccountDataCompleteness::default(),
            fetched_at_ms: 1,
        }
    }

    fn resting_response() -> ExchangeResponse {
        serde_json::from_value(serde_json::json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{ "resting": { "oid": 42_u64 } }]
                }
            }
        }))
        .expect("test exchange response should deserialize")
    }

    fn terminal_with_pending_move() -> (TradingTerminal, Option<u64>) {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.charts.clear();
        terminal
            .charts
            .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
        let order = open_order(42, "100");
        terminal.account_data = Some(account_data_with_order(order.clone()));
        let pending_id = terminal.add_pending_order_modification_indicator(
            TEST_ACCOUNT.to_string(),
            &order,
            "111".to_string(),
        );
        assert!(pending_id.is_some());
        (terminal, pending_id)
    }

    #[test]
    fn move_result_success_carries_confirmed_price_into_local_snapshot() {
        let (mut terminal, pending_id) = terminal_with_pending_move();

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            42,
            pending_id,
            Ok(resting_response()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "111");
        let chart = &terminal.charts.get(&1).expect("chart").chart;
        assert_eq!(chart.active_orders.len(), 1);
        assert_eq!(chart.active_orders[0].limit_px, 111.0);
    }

    #[test]
    fn move_result_failure_keeps_local_price() {
        let (mut terminal, pending_id) = terminal_with_pending_move();

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            42,
            pending_id,
            Err("exchange request failed".to_string()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
        let (message, is_error) = terminal.order_status.expect("status should be set");
        assert!(is_error);
        assert!(message.starts_with("Move modify failed"));
    }

    #[test]
    fn move_result_after_account_switch_skips_status_and_price_update() {
        let (mut terminal, pending_id) = terminal_with_pending_move();
        terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
        terminal.order_status = None;

        let _task = terminal.handle_move_order_modify_result(
            TEST_ACCOUNT.to_string(),
            42,
            pending_id,
            Ok(resting_response()),
        );

        assert!(terminal.pending_order_indicators.is_empty());
        assert!(terminal.order_status.is_none());
        let data = terminal.account_data.as_ref().expect("account data");
        assert_eq!(data.open_orders[0].limit_px, "100");
    }
}
