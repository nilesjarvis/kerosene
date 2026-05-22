use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{
    CHASE_RETRY_COOLDOWN, ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason,
    ExchangeResponse, MIN_CHASE_REPRICE_INTERVAL,
};
use crate::twap_state::ADVANCED_ORDER_GLOBAL_EXCHANGE_INTERVAL;

use iced::Task;
use std::time::Instant;

use super::super::results::result_requires_account_refresh;
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
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        if !self.chase_orders.contains_key(&chase_id) {
            return self.refresh_after_chase_result(should_refresh);
        }
        let lifecycle = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| chase.lifecycle);
        let Some(lifecycle) = lifecycle else {
            return self.refresh_after_chase_result(false);
        };
        if !lifecycle.expects_modify_result(oid) {
            return self.refresh_after_chase_result(false);
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    return self.handle_chase_modify_error(chase_id, oid, resp.summary());
                }
                if resp.is_fully_filled() {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        let filled_size = resp.filled_total_size().unwrap_or(chase.remaining_size);
                        chase.add_filled_size(filled_size);
                    }
                    self.order_status = Some((resp.summary(), false));
                    self.remove_chase_order(chase_id);
                    return self.refresh_after_chase_result(true);
                }

                let stop_status = self.finish_successful_chase_modify(chase_id, oid);
                if let Some((reason, is_error)) = stop_status {
                    return self.stop_chase_by_id_with_reason(chase_id, reason, is_error);
                }
                self.order_status = Some((
                    format!("Chasing (oid {oid}); refreshing account data..."),
                    false,
                ));
                self.refresh_after_chase_result(true)
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
        oid: u64,
    ) -> Option<(String, bool)> {
        let chase = self.chase_orders.get_mut(&chase_id)?;
        chase.record_oid(oid);
        chase.current_oid = Some(oid);
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
