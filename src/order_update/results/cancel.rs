use super::{ExecutionOutcomeKind, classify_execution_result};
use crate::api::{OrderStatusResult, fetch_order_status_by_oid};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::order_execution::cancel_error_indicates_closed_order;
use crate::signing::ExchangeResponse;
use iced::Task;

impl TradingTerminal {
    fn remove_local_open_order(&mut self, account_address: &str, oid: u64, symbol: &str) {
        let Some(data) = self.account_data_for_order_account_mut(account_address) else {
            return;
        };
        let before = data.open_orders.len();
        data.open_orders
            .retain(|order| order.oid != oid || order.coin != symbol);
        if data.open_orders.len() != before {
            self.sync_all_chart_orders();
        }
    }

    fn cancel_order_status_task(
        account_address: String,
        oid: u64,
        symbol: String,
    ) -> Task<Message> {
        Task::perform(
            fetch_order_status_by_oid(account_address.clone(), oid),
            move |result| Message::CancelOrderStatusLoaded {
                account_address: account_address.into(),
                oid,
                symbol,
                result: Box::new(result),
            },
        )
    }

    pub(crate) fn handle_cancel_result(
        &mut self,
        account_address: String,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let cancelled_order = self
            .pending_cancel_indicator_order(pending_indicator_id)
            .or_else(|| {
                self.pending_cancel_status_request
                    .as_ref()
                    .filter(|pending| pending.is_for_account(&account_address))
                    .map(|pending| (pending.oid(), pending.symbol().to_string()))
            });
        self.clear_pending_order_indicator(pending_indicator_id);
        if !self.connected_order_account_matches(&account_address) {
            if self
                .pending_cancel_status_request
                .as_ref()
                .is_some_and(|pending| pending.is_for_account(&account_address))
            {
                self.pending_cancel_status_request = None;
            }
            return Task::none();
        }
        let cancelled_oid = cancelled_order.as_ref().map(|(oid, _)| *oid);
        let outcome = classify_execution_result(result);
        let verification_prefix = if matches!(
            outcome.kind,
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown
        ) {
            Some("Cancel status unknown")
        } else if outcome.kind == ExecutionOutcomeKind::Rejected
            && cancel_error_indicates_closed_order(&outcome.status)
        {
            Some("Cancel may have already resolved")
        } else {
            None
        };
        if let Some(prefix) = verification_prefix {
            let status_task = cancelled_order.map_or_else(Task::none, |(oid, symbol)| {
                Self::cancel_order_status_task(account_address, oid, symbol)
            });
            let order_label = cancelled_oid
                .map(|oid| format!(" for order {oid}"))
                .unwrap_or_default();
            self.set_order_status(
                format!(
                    "{prefix}{order_label}: {}; checking orderStatus and refreshing account data",
                    outcome.status
                ),
                true,
            );
            return Task::batch([self.refresh_account_data(), status_task]);
        }
        self.pending_cancel_status_request = None;
        // Drop the order from the local snapshot on a confirmed cancel so the
        // ack does not resurrect an interactive line for an order the exchange
        // has already removed; the next authoritative update wins regardless.
        if outcome.kind == ExecutionOutcomeKind::Cancelled
            && let Some((oid, symbol)) = cancelled_order
        {
            self.remove_local_open_order(&account_address, oid, &symbol);
        }
        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn handle_cancel_order_status_result(
        &mut self,
        account_address: String,
        oid: u64,
        symbol: String,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let request_matches = self
            .pending_cancel_status_request
            .as_ref()
            .is_some_and(|pending| pending.matches(&account_address, oid, &symbol));
        if !request_matches {
            return Task::none();
        }

        if !self.connected_order_account_matches(&account_address) {
            self.pending_cancel_status_request = None;
            return Task::none();
        }

        match result {
            Ok(status) if status.is_open() => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: orderStatus reports open ({}); refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_filled() => {
                self.pending_cancel_status_request = None;
                self.remove_local_open_order(&account_address, oid, &symbol);
                self.set_order_status(
                    format!(
                        "Cancel did not prevent fill for order {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_no_fill_terminal() => {
                self.pending_cancel_status_request = None;
                self.remove_local_open_order(&account_address, oid, &symbol);
                self.set_order_status(
                    format!(
                        "Cancel resolved for order {oid}: orderStatus reports {}; refreshing account data",
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_missing() => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: orderStatus returned {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Err(error) => {
                let error = redact_sensitive_response_text(&error);
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: {error}; refreshing account data"
                    ),
                    true,
                );
            }
        }

        self.refresh_account_data()
    }
}
