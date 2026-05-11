use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{CHASE_RATE_LIMIT_COOLDOWN, ChasePendingOp, ExchangeResponse};

use iced::Task;
use std::time::Instant;

use super::super::results::result_requires_account_refresh;

#[cfg(test)]
mod tests;

fn chase_terminal_modify_error(summary: &str) -> bool {
    let summary = summary.to_ascii_lowercase();
    summary.contains("cannot modify")
        && (summary.contains("cancel") || summary.contains("cancled") || summary.contains("filled"))
}

fn chase_rate_limit_error(summary: &str) -> bool {
    let summary = summary.to_ascii_lowercase();
    summary.contains("rate limit")
        || summary.contains("ratelimit")
        || summary.contains("too many requests")
        || summary.contains("429")
}

impl TradingTerminal {
    pub(crate) fn handle_chase_modify_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        requested_price: f64,
        requested_price_wire: String,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        let mut refresh_after_result = should_refresh;
        if self
            .active_chase
            .as_ref()
            .is_none_or(|chase| chase.id != chase_id)
        {
            return self.refresh_after_chase_result(should_refresh);
        }
        if !self.active_chase.as_ref().is_some_and(|chase| {
            matches!(chase.pending_op, Some(ChasePendingOp::Modify { oid: pending_oid }) if pending_oid == oid)
        }) {
            return self.refresh_after_chase_result(should_refresh);
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    let summary = resp.summary();
                    let mut stop_after_modify = false;
                    let mut terminal_modify_error = false;
                    let mut rate_limited = false;
                    if let Some(chase) = &mut self.active_chase {
                        chase.pending_op = None;
                        stop_after_modify = chase.stop_requested;
                        if chase_terminal_modify_error(&summary) {
                            chase.current_oid = Some(oid);
                            chase.missing_open_order_refresh_requested = true;
                            terminal_modify_error = true;
                        } else if chase_rate_limit_error(&summary) {
                            chase.last_reprice_at =
                                Some(Instant::now() + CHASE_RATE_LIMIT_COOLDOWN);
                            rate_limited = true;
                        }
                    }
                    if terminal_modify_error {
                        return self.check_chase_order_status(
                            oid,
                            "Chase checking order status: order was filled or cancelled before the modify settled",
                        );
                    }
                    if rate_limited {
                        self.order_status = Some((
                            format!(
                                "Chase paused: exchange rate limit; will retry in about {}s",
                                CHASE_RATE_LIMIT_COOLDOWN.as_secs()
                            ),
                            true,
                        ));
                        if stop_after_modify {
                            let (reason, is_error) = self
                                .active_chase
                                .as_ref()
                                .and_then(|chase| chase.stop_reason.clone())
                                .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                            return self.stop_chase_with_reason(reason, is_error);
                        }
                        return Task::none();
                    }
                    self.order_status = Some((format!("Chase modify failed: {summary}"), true));
                    if stop_after_modify {
                        let (reason, is_error) = self
                            .active_chase
                            .as_ref()
                            .and_then(|chase| chase.stop_reason.clone())
                            .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                        return self.stop_chase_with_reason(reason, is_error);
                    }
                } else if resp.is_fully_filled() {
                    self.order_status = Some((resp.summary(), false));
                    self.active_chase = None;
                } else {
                    refresh_after_result = false;
                    let mut stop_after_modify = false;
                    if let Some(chase) = &mut self.active_chase {
                        let current_oid = resp.order_oid().unwrap_or(oid);
                        chase.current_oid = Some(current_oid);
                        chase.current_price = requested_price;
                        chase.current_price_wire = requested_price_wire;
                        chase.pending_op = None;
                        chase.oid_confirmed = false;
                        chase.cancel_retries = 0;
                        chase.missing_open_order_refresh_requested = false;
                        stop_after_modify = chase.stop_requested;
                        self.order_status =
                            Some((format!("Chasing (oid {current_oid})..."), false));
                    }
                    if stop_after_modify {
                        let (reason, is_error) = self
                            .active_chase
                            .as_ref()
                            .and_then(|chase| chase.stop_reason.clone())
                            .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                        return self.stop_chase_with_reason(reason, is_error);
                    }
                }
            }
            Err(e) => {
                let mut stop_after_modify = false;
                let mut rate_limited = false;
                if let Some(chase) = &mut self.active_chase {
                    chase.pending_op = None;
                    stop_after_modify = chase.stop_requested;
                    if chase_rate_limit_error(&e) {
                        chase.last_reprice_at = Some(Instant::now() + CHASE_RATE_LIMIT_COOLDOWN);
                        rate_limited = true;
                    }
                }
                if stop_after_modify {
                    let (reason, is_error) = self
                        .active_chase
                        .as_ref()
                        .and_then(|chase| chase.stop_reason.clone())
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    return self.stop_chase_with_reason(reason, is_error);
                }
                if rate_limited {
                    self.order_status = Some((
                        format!(
                            "Chase paused: exchange rate limit; will retry in about {}s",
                            CHASE_RATE_LIMIT_COOLDOWN.as_secs()
                        ),
                        true,
                    ));
                    return Task::none();
                }
                return self.check_chase_order_status(
                    oid,
                    format!("Chase checking order status: modify response was not confirmed ({e})"),
                );
            }
        }

        self.refresh_after_chase_result(refresh_after_result)
    }
}
