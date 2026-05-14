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
        if !self.chase_orders.contains_key(&chase_id) {
            return self.refresh_after_chase_result(should_refresh);
        }
        let pending_op = self
            .chase_orders
            .get(&chase_id)
            .and_then(|chase| chase.pending_op);
        let Some(pending_op) = pending_op else {
            return self.refresh_after_chase_result(should_refresh);
        };
        let pending_oid = match pending_op {
            ChasePendingOp::Cancel { oid } | ChasePendingOp::CancelForReprice { oid } => oid,
            _ => return self.refresh_after_chase_result(should_refresh),
        };
        if pending_oid != oid {
            return self.refresh_after_chase_result(should_refresh);
        }

        if matches!(pending_op, ChasePendingOp::CancelForReprice { .. }) {
            return self.handle_chase_reprice_cancel_result(chase_id, oid, result);
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    let summary = resp.summary();
                    if chase_terminal_cancel_error(&summary) {
                        return self.check_chase_order_status(
                            chase_id,
                            oid,
                            "Chase checking order status: order was filled or cancelled before the cancel settled",
                        );
                    }
                    self.handle_chase_cancel_error(chase_id, summary, false);
                } else {
                    let stop_status = self.chase_orders.get(&chase_id).and_then(|chase| {
                        chase
                            .stop_reason
                            .clone()
                            .or_else(|| Some(("Chase stopped".to_string(), false)))
                    });
                    if let Some((message, is_error)) = stop_status {
                        self.order_status = Some((message, is_error));
                    }
                    self.remove_chase_order(chase_id);
                    return self.refresh_after_chase_result(true);
                }
            }
            Err(e) => {
                return self.handle_chase_uncertain_cancel_result(chase_id, oid, e);
            }
        }

        self.refresh_after_chase_result(should_refresh)
    }

    fn handle_chase_reprice_cancel_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        match result {
            Ok(resp) => {
                if resp.is_error() {
                    let summary = resp.summary();
                    if chase_terminal_cancel_error(&summary) {
                        self.prepare_chase_reprice_reconciliation(chase_id, oid);
                        self.order_status = Some((
                            "Chase checking order status: order was filled or cancelled before the reprice cancel settled"
                                .into(),
                            false,
                        ));
                        return self.refresh_account_data();
                    }
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.pending_op = None;
                    }
                    self.order_status =
                        Some((format!("Chase reprice cancel failed: {summary}"), true));
                    self.refresh_after_chase_result(should_refresh)
                } else {
                    let stop_status = self.chase_orders.get(&chase_id).and_then(|chase| {
                        chase.stop_requested.then(|| {
                            chase
                                .stop_reason
                                .clone()
                                .unwrap_or_else(|| ("Chase stopped".to_string(), false))
                        })
                    });
                    if let Some((reason, is_error)) = stop_status {
                        self.order_status = Some((reason, is_error));
                        self.remove_chase_order(chase_id);
                        return self.refresh_after_chase_result(true);
                    }

                    self.prepare_chase_reprice_reconciliation(chase_id, oid);
                    self.order_status = Some((
                        "Chase repricing: reconciling fills before replacement".into(),
                        false,
                    ));
                    self.refresh_account_data()
                }
            }
            Err(e) => {
                self.prepare_chase_reprice_reconciliation(chase_id, oid);
                self.order_status = Some((
                    format!(
                        "Chase checking order status: reprice cancel response was not confirmed ({e})"
                    ),
                    false,
                ));
                self.refresh_account_data()
            }
        }
    }

    fn prepare_chase_reprice_reconciliation(&mut self, chase_id: u64, oid: u64) {
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.record_oid(oid);
            chase.current_oid = Some(oid);
            chase.pending_op = None;
            chase.oid_confirmed = true;
            chase.missing_open_order_refresh_requested = true;
        }
    }

    fn handle_chase_uncertain_cancel_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        message: String,
    ) -> Task<Message> {
        let mut retry_count = 0;
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
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
                self.remove_chase_order(chase_id);
                return self.refresh_account_data();
            }
        }

        self.check_chase_order_status(
            chase_id,
            oid,
            format!(
                "Chase checking order status: cancel response was not confirmed (attempt {retry_count}/{}: {message})",
                signing::MAX_CHASE_CANCEL_RETRIES
            ),
        )
    }

    fn handle_chase_cancel_error(
        &mut self,
        chase_id: u64,
        message: String,
        include_last_on_stop: bool,
    ) {
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
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
                self.remove_chase_order(chase_id);
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
