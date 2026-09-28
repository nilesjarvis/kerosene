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
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown => {
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
mod tests;
