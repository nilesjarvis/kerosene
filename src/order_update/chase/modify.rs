use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::signing::{
    CHASE_RETRY_COOLDOWN, ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason,
    ExchangeResponse, MIN_CHASE_REPRICE_INTERVAL,
};
use crate::twap_state::ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL;

use iced::Task;
use std::time::Instant;

use super::cancel::chase_terminal_cancel_error;

#[cfg(test)]
mod tests;

fn chase_retryable_exchange_error(summary: &str) -> bool {
    let summary = summary.to_ascii_lowercase();
    summary.contains("rate limit")
        || summary.contains("ratelimit")
        || summary.contains("too many requests")
        || summary.contains("429")
        || summary.contains("temporarily")
        || summary.contains("unavailable")
        || summary.contains("overloaded")
        || summary.contains("try again")
}

fn cooldown_marker(now: Instant, gate: std::time::Duration) -> Instant {
    now + CHASE_RETRY_COOLDOWN.saturating_sub(gate)
}

impl TradingTerminal {
    pub(crate) fn handle_chase_modify_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        reprice_count: u32,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
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
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        // The exchange may retain the same OID across multiple modifies.
        // Require the dispatch-time sequence as well as the lifecycle/OID.
        if chase.reprice_count != reprice_count || !chase.lifecycle.expects_modify_result(oid) {
            return Task::none();
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    return self.handle_chase_modify_error(chase_id, oid, resp.summary());
                }
                if !resp.is_confirmed_modify_result() {
                    return self.check_chase_order_status(
                        chase_id,
                        oid,
                        format!(
                            "Chase checking order status: modify response was not confirmed ({})",
                            resp.summary()
                        ),
                    );
                }
                if resp.is_fully_filled() {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        let filled_size = resp.filled_total_size().unwrap_or(chase.remaining_size);
                        chase.add_filled_size(filled_size);
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder,
                        };
                    }
                    self.order_status = Some((resp.summary(), false));
                    return self.refresh_after_chase_result_for_order_account(
                        true,
                        &chase_account_address,
                    );
                }

                let resting_oid = resp.order_oid().unwrap_or(oid);
                let stop_status = self.finish_successful_chase_modify(chase_id, oid, resting_oid);
                if let Some((reason, is_error)) = stop_status {
                    return self.stop_chase_by_id_with_reason(chase_id, reason, is_error);
                }
                self.order_status = Some((
                    format!("Chasing (oid {resting_oid}); refreshing account data..."),
                    false,
                ));
                self.refresh_after_chase_result_for_order_account(true, &chase_account_address)
            }
            Err(e) => self.check_chase_order_status(
                chase_id,
                oid,
                format!("Chase checking order status: modify response was not confirmed ({e})"),
            ),
        }
    }

    fn handle_chase_modify_error(
        &mut self,
        chase_id: u64,
        oid: u64,
        summary: String,
    ) -> Task<Message> {
        let summary = redact_sensitive_response_text(&summary);
        if chase_terminal_cancel_error(&summary) {
            return self.check_chase_order_status(
                chase_id,
                oid,
                concat!(
                    "Chase checking order status: order was filled or cancelled before ",
                    "the modify settled"
                ),
            );
        }

        let now = Instant::now();
        let mut apply_global_cooldown = false;
        let stop_status = if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            if chase_retryable_exchange_error(&summary) {
                let was_stopping = chase.lifecycle.is_stopping();
                chase.last_reprice_at = Some(cooldown_marker(now, MIN_CHASE_REPRICE_INTERVAL));
                if was_stopping {
                    chase.lifecycle = ChaseLifecycle::Resting;
                } else {
                    chase.lifecycle = ChaseLifecycle::Queued {
                        action: ChaseQueuedAction::Reprice,
                    };
                }
                apply_global_cooldown = true;
                was_stopping.then(|| {
                    chase
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false))
                })
            } else {
                chase.last_reprice_at = Some(now);
                chase.desired_price = None;
                // The exchange definitively rejected the modify, so no result
                // is in flight and the order still rests at its old price.
                // Return to Resting so the stop below cancels the resting
                // order instead of parking in Stopping::AwaitingModify waiting
                // on a modify result that has already been consumed.
                chase.record_oid(oid);
                chase.current_oid = Some(oid);
                chase.lifecycle = ChaseLifecycle::Resting;
                Some((format!("Chase stopped: modify failed: {summary}"), true))
            }
        } else {
            None
        };
        if apply_global_cooldown {
            self.last_advanced_exchange_request_at = Some(cooldown_marker(
                now,
                ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL,
            ));
        }

        if let Some((reason, is_error)) = stop_status {
            return self.stop_chase_by_id_with_reason(chase_id, reason, is_error);
        }

        self.order_status = Some((format!("Chase modify delayed: {summary}"), true));
        Task::none()
    }

    fn finish_successful_chase_modify(
        &mut self,
        chase_id: u64,
        request_oid: u64,
        resting_oid: u64,
    ) -> Option<(String, bool)> {
        let chase = self.chase_orders.get_mut(&chase_id)?;
        chase.record_oid(request_oid);
        // Hyperliquid modifies have kept the oid stable so far, but adopt the
        // oid echoed in the response in case a modify ever re-keys the order;
        // tracking only the request oid would leave reconciliation following
        // a dead oid while the replacement rests unmanaged.
        chase.record_oid(resting_oid);
        chase.current_oid = Some(resting_oid);
        let was_stopping = chase.lifecycle.is_stopping();
        chase.lifecycle = ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Modify,
        };
        chase.cancel_retries = 0;
        was_stopping.then(|| {
            chase
                .stop_reason
                .clone()
                .unwrap_or_else(|| ("Chase stopped".to_string(), false))
        })
    }
}
