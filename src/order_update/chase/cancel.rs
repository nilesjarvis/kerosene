use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::signing::{self, ChaseLifecycle, ChaseStopPhase, ExchangeResponse};

use iced::Task;
use std::time::Instant;

use super::super::results::result_requires_account_refresh;

#[cfg(test)]
mod tests;

pub(in crate::order_update) fn chase_terminal_cancel_error(summary: &str) -> bool {
    let summary = summary.to_ascii_lowercase();
    summary.contains("filled")
        || summary.contains("canceled")
        || summary.contains("cancelled")
        || summary.contains("cancled")
        || summary.contains("never placed")
        || summary.contains("not found")
        || summary.contains("does not exist")
        || summary.contains("no open order")
        || summary.contains("no longer open")
}

impl TradingTerminal {
    pub(crate) fn handle_chase_cancel_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        if !self.chase_orders.contains_key(&chase_id) {
            return Task::none();
        }
        let Some(chase_account_address) = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| chase.account_address.clone())
        else {
            return Task::none();
        };
        let lifecycle = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| chase.lifecycle);
        let Some(lifecycle) = lifecycle else {
            return Task::none();
        };
        if !lifecycle.expects_cancel_result(oid) {
            return self.refresh_after_chase_result_for_order_account(
                should_refresh,
                &chase_account_address,
            );
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    let summary = resp.summary();
                    if chase_terminal_cancel_error(&summary) {
                        return self.check_chase_order_status(
                            chase_id,
                            oid,
                            concat!(
                                "Chase checking order status: order was filled or cancelled ",
                                "before the cancel settled"
                            ),
                        );
                    }
                    self.handle_chase_cancel_error(chase_id, oid, summary, false);
                    if self.archive_disconnected_manual_check_chase(chase_id) {
                        return Task::none();
                    }
                } else if resp.is_confirmed_cancel_result() {
                    let stop_status = self
                        .chase_orders
                        .get(&chase_id)
                        .and_then(|chase| chase.stop_reason.clone())
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    self.set_order_status_toast_on_error(stop_status.0.clone(), stop_status.1);
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        chase.current_oid = Some(oid);
                        chase.cancel_retries = 0;
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { oid },
                        };
                    }
                    if self.archive_disconnected_stopping_chase(chase_id, stop_status.0) {
                        return Task::none();
                    }
                    return self.refresh_after_chase_result_for_order_account(
                        true,
                        &chase_account_address,
                    );
                } else {
                    return self.handle_chase_uncertain_cancel_result(
                        chase_id,
                        oid,
                        resp.summary(),
                    );
                }
            }
            Err(e) => {
                return self.handle_chase_uncertain_cancel_result(chase_id, oid, e);
            }
        }

        self.refresh_after_chase_result_for_order_account(should_refresh, &chase_account_address)
    }

    fn handle_chase_uncertain_cancel_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        message: String,
    ) -> Task<Message> {
        let message = redact_sensitive_response_text(&message);
        let mut retry_count = 0;
        let chase_account_address = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| chase.account_address.clone());
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.cancel_retries += 1;
            retry_count = chase.cancel_retries;
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { oid },
            };
            if retry_count >= signing::MAX_CHASE_CANCEL_RETRIES {
                let manual_status = format!(
                    concat!(
                        "Chase requires manual check: cancel status could not be confirmed ",
                        "after {} attempts; check open orders (last: {})"
                    ),
                    signing::MAX_CHASE_CANCEL_RETRIES,
                    message
                );
                chase.stop_reason = Some((manual_status.clone(), true));
                self.set_order_status(manual_status, true);
                if self.archive_disconnected_manual_check_chase(chase_id) {
                    return Task::none();
                }
                return chase_account_address
                    .as_deref()
                    .map_or_else(Task::none, |address| {
                        self.refresh_account_data_for_order_account(address)
                    });
            }
        }

        self.check_chase_order_status(
            chase_id,
            oid,
            format!(
                concat!(
                    "Chase checking order status: cancel response was not confirmed ",
                    "(attempt {}/{}: {})"
                ),
                retry_count,
                signing::MAX_CHASE_CANCEL_RETRIES,
                message
            ),
        )
    }

    fn archive_disconnected_manual_check_chase(&mut self, chase_id: u64) -> bool {
        let summary = self
            .chase_orders
            .get(&chase_id)
            .and_then(|chase| chase.stop_reason.as_ref())
            .filter(|(message, is_error)| *is_error && message.contains("manual check"))
            .map(|(message, _)| message.clone());
        summary.is_some_and(|summary| self.archive_disconnected_stopping_chase(chase_id, summary))
    }

    fn handle_chase_cancel_error(
        &mut self,
        chase_id: u64,
        oid: u64,
        message: String,
        include_last_on_stop: bool,
    ) {
        let message = redact_sensitive_response_text(&message);
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.cancel_retries += 1;
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { oid },
            };
            chase.last_reprice_at = Some(Instant::now());
            if chase.cancel_retries >= signing::MAX_CHASE_CANCEL_RETRIES {
                let suffix = if include_last_on_stop {
                    format!(" (last: {message})")
                } else {
                    String::new()
                };
                let manual_status = format!(
                    "Chase requires manual check: cancel failed {} times{}; check open orders",
                    chase.cancel_retries, suffix
                );
                chase.stop_reason = Some((manual_status.clone(), true));
                self.set_order_status(manual_status, true);
            } else {
                let retry_status = format!(
                    "Chase cancel error (retry {}/{}): {}",
                    chase.cancel_retries,
                    signing::MAX_CHASE_CANCEL_RETRIES,
                    message
                );
                self.set_order_status(retry_status, true);
            }
        }
    }
}
