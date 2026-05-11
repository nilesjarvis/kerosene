use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{self, ChasePendingOp, ExchangeResponse};

use iced::Task;

use super::super::results::result_requires_account_refresh;

#[cfg(test)]
mod tests;

fn chase_terminal_cancel_error(summary: &str) -> bool {
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
        if self
            .active_chase
            .as_ref()
            .is_none_or(|chase| chase.id != chase_id)
        {
            return self.refresh_after_chase_result(should_refresh);
        }
        if !self.active_chase.as_ref().is_some_and(|chase| {
            matches!(chase.pending_op, Some(ChasePendingOp::Cancel { oid: pending_oid }) if pending_oid == oid)
        }) {
            return self.refresh_after_chase_result(should_refresh);
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    let summary = resp.summary();
                    if chase_terminal_cancel_error(&summary) {
                        return self.check_chase_order_status(
                            oid,
                            "Chase checking order status: order was filled or cancelled before the cancel settled",
                        );
                    }
                    self.handle_chase_cancel_error(summary, false);
                } else {
                    self.pending_order_action = None;
                    let stop_status = self.active_chase.as_ref().and_then(|chase| {
                        chase
                            .stop_reason
                            .clone()
                            .or_else(|| Some(("Chase stopped".to_string(), false)))
                    });
                    if let Some((message, is_error)) = stop_status {
                        self.order_status = Some((message, is_error));
                    }
                    self.active_chase = None;
                    return self.refresh_after_chase_result(true);
                }
            }
            Err(e) => {
                return self.handle_chase_uncertain_cancel_result(oid, e);
            }
        }

        self.refresh_after_chase_result(should_refresh)
    }

    fn handle_chase_uncertain_cancel_result(&mut self, oid: u64, message: String) -> Task<Message> {
        let mut retry_count = 0;
        if let Some(chase) = &mut self.active_chase {
            chase.cancel_retries += 1;
            retry_count = chase.cancel_retries;
            if retry_count >= signing::MAX_CHASE_CANCEL_RETRIES {
                self.order_status = Some((
                    format!(
                        "Chase stopped: cancel status could not be confirmed after {} attempts; check open orders (last: {message})",
                        signing::MAX_CHASE_CANCEL_RETRIES
                    ),
                    true,
                ));
                self.active_chase = None;
                return self.refresh_account_data();
            }
        }

        self.check_chase_order_status(
            oid,
            format!(
                "Chase checking order status: cancel response was not confirmed (attempt {retry_count}/{}: {message})",
                signing::MAX_CHASE_CANCEL_RETRIES
            ),
        )
    }

    fn handle_chase_cancel_error(&mut self, message: String, include_last_on_stop: bool) {
        if let Some(chase) = &mut self.active_chase {
            chase.cancel_retries += 1;
            chase.pending_op = None;
            if chase.cancel_retries >= signing::MAX_CHASE_CANCEL_RETRIES {
                let suffix = if include_last_on_stop {
                    format!(" (last: {message})")
                } else {
                    String::new()
                };
                self.order_status = Some((
                    format!(
                        "Chase stopped: cancel failed {} times{}; check open orders",
                        chase.cancel_retries, suffix
                    ),
                    true,
                ));
                self.active_chase = None;
            } else {
                self.order_status = Some((
                    format!(
                        "Chase cancel error (retry {}/{}): {}",
                        chase.cancel_retries,
                        signing::MAX_CHASE_CANCEL_RETRIES,
                        message
                    ),
                    true,
                ));
            }
        }
    }
}
