use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{
    CHASE_RETRY_COOLDOWN, ChasePendingOp, ExchangeResponse, MIN_CHASE_REPRICE_INTERVAL,
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
        let pending_op = self
            .chase_orders
            .get(&chase_id)
            .and_then(|chase| chase.pending_op);
        let Some(ChasePendingOp::Modify { oid: pending_oid }) = pending_op else {
            return self.refresh_after_chase_result(false);
        };
        if pending_oid != oid {
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
                self.order_status = Some((format!("Chasing (oid {oid})..."), false));
                Task::none()
            }
            Err(e) => {
                self.check_chase_order_status(
                    chase_id,
                    oid,
                    format!(
                        "Chase checking order status: modify response was not confirmed ({e})"
                    ),
                )
            }
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
            chase.pending_op = None;
            if chase_retryable_exchange_error(&summary) {
                chase.last_reprice_at = Some(cooldown_marker(now, MIN_CHASE_REPRICE_INTERVAL));
                apply_global_cooldown = true;
                chase.stop_requested.then(|| {
                    chase
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false))
                })
            } else {
                chase.last_reprice_at = Some(now);
                chase.pending_best_price = None;
                chase.pending_size_correction = false;
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
        if let Some(best) = chase.pending_best_price
            && let Some((rounded_best, price_wire)) = chase.rounded_price(best)
        {
            chase.current_price = rounded_best;
            chase.current_price_wire = price_wire;
        }
        chase.pending_op = None;
        chase.pending_best_price = None;
        chase.pending_size_correction = false;
        chase.cancel_retries = 0;
        chase.oid_confirmed = false;
        chase.missing_open_order_refresh_requested = false;
        chase.stop_requested.then(|| {
            chase
                .stop_reason
                .clone()
                .unwrap_or_else(|| ("Chase stopped".to_string(), false))
        })
    }
}
