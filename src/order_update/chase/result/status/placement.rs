use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};

use iced::Task;
use std::time::Instant;

use super::returned_cloid_mismatches;

// ---------------------------------------------------------------------------
// Placement Status Results
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_chase_order_status_result(
        &mut self,
        chase_id: u64,
        cloid: String,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        let expects_place_status = chase.lifecycle.expects_place_result()
            || matches!(
                chase.lifecycle,
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Placement
                }
            );
        if !expects_place_status || chase.current_cloid.as_deref() != Some(cloid.as_str()) {
            return Task::none();
        }
        let chase_account_address = chase.account_address.clone();

        match result {
            Ok(status) if returned_cloid_mismatches(&status, &cloid) => {
                self.order_status = Some((
                    format!(
                        concat!(
                            "Chase placement status ignored: response cloid did not match ",
                            "request {} ({})"
                        ),
                        cloid, status.raw_summary
                    ),
                    true,
                ));
                Task::none()
            }
            Ok(status) if status.is_open() => {
                let Some(oid) = status.oid else {
                    let summary = format!(
                        concat!(
                            "Chase stopped: placement status open for {} but no order id ",
                            "was returned"
                        ),
                        cloid
                    );
                    self.fail_chase_order(chase_id, summary);
                    return self.refresh_account_data_for_order_account(&chase_account_address);
                };
                let stop_status = if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    let was_stopping = chase.lifecycle.is_stopping();
                    chase.record_oid(oid);
                    chase.current_oid = Some(oid);
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::Placement,
                    };
                    chase.cancel_retries = 0;
                    was_stopping.then(|| {
                        chase
                            .stop_reason
                            .clone()
                            .unwrap_or_else(|| ("Chase stopped".to_string(), false))
                    })
                } else {
                    None
                };
                if let Some((reason, is_error)) = stop_status {
                    return self.stop_chase_by_id_with_reason(chase_id, reason, is_error);
                }
                self.order_status = Some((
                    format!(
                        "Chase placement confirmed by orderStatus: {}; refreshing account data",
                        status.raw_summary
                    ),
                    false,
                ));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_filled() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    if let Some(oid) = status.oid {
                        chase.record_oid(oid);
                        chase.current_oid = Some(oid);
                    }
                    let filled_size = chase.remaining_size;
                    chase.add_filled_size(filled_size);
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                }
                let summary = format!(
                    "Chase placement filled according to orderStatus: {}; refreshing account data",
                    status.raw_summary
                );
                self.order_status = Some((summary.clone(), false));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_definitive_no_fill_terminal() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id)
                    && let Some(oid) = status.oid
                {
                    chase.record_oid(oid);
                }
                self.finish_definitive_chase_place_failure(
                    chase_id,
                    format!(
                        "Chase stopped: placement resolved without fill as {}",
                        status.raw_summary
                    ),
                );
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_no_fill_terminal() => {
                let summary = format!(
                    concat!(
                        "Chase stopped: placement is no longer open ({}); waiting for account ",
                        "reconciliation"
                    ),
                    status.raw_summary
                );
                let was_stopping = self
                    .chase_orders
                    .get(&chase_id)
                    .is_some_and(|chase| chase.lifecycle.is_stopping());
                if status.oid.is_none() && !was_stopping {
                    self.finish_definitive_chase_place_failure(chase_id, summary);
                    return self.refresh_account_data_for_order_account(&chase_account_address);
                }
                let mut display_status = (summary.clone(), true);
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    if was_stopping {
                        display_status = chase
                            .stop_reason
                            .clone()
                            .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    }
                    if let Some(oid) = status.oid {
                        chase.record_oid(oid);
                        chase.current_oid = Some(oid);
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { oid },
                        };
                    } else if was_stopping {
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::AwaitingPlace,
                        };
                    } else {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::Placement,
                        };
                    }
                    chase.desired_price = None;
                    chase.stop_reason = Some(display_status.clone());
                    chase.last_reprice_at = Some(Instant::now());
                }
                self.order_status = Some(display_status);
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_missing() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    if chase.lifecycle.is_stopping() {
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::AwaitingPlace,
                        };
                    } else {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::Placement,
                        };
                    }
                    chase.last_reprice_at = Some(Instant::now());
                }
                self.order_status = Some((
                    format!(
                        "Chase placement status ambiguous for {cloid}: {}; keeping chase state",
                        status.raw_summary
                    ),
                    true,
                ));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) => {
                let summary = format!(
                    "Chase stopped: placement status for {cloid} was {}",
                    status.raw_summary
                );
                self.fail_chase_order(chase_id, summary);
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Err(error) => {
                let error = redact_sensitive_response_text(&error);
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    if chase.lifecycle.is_stopping() {
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::AwaitingPlace,
                        };
                    } else {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::Placement,
                        };
                    }
                    chase.last_reprice_at = Some(Instant::now());
                }
                self.order_status = Some((
                    format!(
                        concat!(
                            "Chase placement status still uncertain for {}: {}; ",
                            "keeping chase state"
                        ),
                        cloid, error
                    ),
                    true,
                ));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
        }
    }
}
