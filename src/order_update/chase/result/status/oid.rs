use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::order_execution::order_account_addresses_match;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};

use iced::Task;
use std::time::Instant;

use super::returned_oid_mismatches;

// ---------------------------------------------------------------------------
// OID Status Results
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_chase_order_oid_status_result(
        &mut self,
        chase_id: u64,
        oid: u64,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        if chase.current_oid != Some(oid) {
            return Task::none();
        }
        let chase_account_address = chase.account_address.clone();
        let cancel_already_in_flight = matches!(
            chase.lifecycle,
            ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::Canceling { oid: pending_oid },
            } if pending_oid == oid
        );

        let can_refresh_chase_account = self
            .connected_address
            .as_deref()
            .is_some_and(|address| order_account_addresses_match(address, &chase_account_address));

        match result {
            Ok(status) if returned_oid_mismatches(&status, oid) => {
                self.order_status = Some((
                    format!(
                        concat!(
                            "Chase order status ignored: response oid did not match ",
                            "request {} ({})"
                        ),
                        oid, status.raw_summary
                    ),
                    true,
                ));
                Task::none()
            }
            Ok(status) if status.is_open() => {
                if cancel_already_in_flight {
                    let is_error = chase
                        .stop_reason
                        .as_ref()
                        .map(|(_, is_error)| *is_error)
                        .unwrap_or(false);
                    self.order_status = Some((
                        format!(
                            "Chase stopping: cancel already in flight for order {oid}; orderStatus still reports open ({})",
                            status.raw_summary
                        ),
                        is_error,
                    ));
                    return Task::none();
                }
                chase.record_oid(oid);
                if chase.lifecycle.is_stopping() {
                    chase.lifecycle = ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { oid },
                    };
                } else {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::Modify,
                    };
                }
                if chase.lifecycle.is_stopping() {
                    let (reason, is_error) = chase
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    return self.stop_chase_by_id_with_reason(chase_id, reason, is_error);
                }
                self.order_status = Some((
                    format!(
                        "Chase order status confirmed open: {}; refreshing account data",
                        status.raw_summary
                    ),
                    false,
                ));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_filled() => {
                if chase.lifecycle.is_stopping() && !can_refresh_chase_account {
                    let summary = format!(
                        "Chase stopped: order filled according to orderStatus ({})",
                        status.raw_summary
                    );
                    self.order_status = Some((summary.clone(), false));
                    self.archive_disconnected_stopping_chase(chase_id, summary);
                    return Task::none();
                }
                chase.record_oid(oid);
                let filled_size = chase.remaining_size;
                chase.add_filled_size(filled_size);
                chase.lifecycle = ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrder,
                };
                let summary = format!(
                    "Chase order filled according to orderStatus: {}; refreshing account data",
                    status.raw_summary
                );
                self.order_status = Some((summary, false));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_definitive_no_fill_terminal() => {
                if chase.lifecycle.is_stopping() {
                    let (message, is_error) = chase
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    self.order_status = Some((message.clone(), is_error));
                    chase.record_oid(oid);
                    chase.current_oid = Some(oid);
                    chase.lifecycle = ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { oid },
                    };
                    if self.archive_disconnected_stopping_chase(chase_id, message) {
                        return Task::none();
                    }
                    return self.refresh_account_data_for_order_account(&chase_account_address);
                }
                chase.lifecycle = ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrderResolvedNoFill,
                };
                self.order_status = Some((
                    format!(
                        "Chase checking account state: orderStatus resolved without fill as {}",
                        status.raw_summary
                    ),
                    false,
                ));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_no_fill_terminal() => {
                let was_stopping = chase.lifecycle.is_stopping();
                let summary = format!(
                    "Chase stopped: order no longer open ({}); no replacement will be placed",
                    status.raw_summary
                );
                chase.record_oid(oid);
                chase.current_oid = Some(oid);
                chase.desired_price = None;
                chase.stop_reason = Some((summary.clone(), true));
                chase.lifecycle = ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::VerifyingCancel { oid },
                };
                self.order_status = Some((summary.clone(), true));
                if was_stopping && self.archive_disconnected_stopping_chase(chase_id, summary) {
                    return Task::none();
                }
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Ok(status) if status.is_missing() => {
                if chase.lifecycle.is_stopping() {
                    let archive_summary = (!can_refresh_chase_account).then(|| {
                        format!(
                            concat!(
                                "Chase stopped: orderStatus did not find previous account ",
                                "order {} ({})"
                            ),
                            oid, status.raw_summary
                        )
                    });
                    chase.record_oid(oid);
                    chase.current_oid = Some(oid);
                    chase.lifecycle = ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { oid },
                    };
                    chase.last_reprice_at = Some(Instant::now());
                    if let Some(summary) = &archive_summary {
                        chase.stop_reason = Some((summary.clone(), true));
                    }
                    if let Some(summary) = archive_summary {
                        self.order_status = Some((summary.clone(), true));
                        self.archive_disconnected_stopping_chase(chase_id, summary);
                        return Task::none();
                    }
                    self.order_status = Some((
                        format!(
                            concat!(
                                "Chase stop status ambiguous for oid {}: {}; verifying ",
                                "account state"
                            ),
                            oid, status.raw_summary
                        ),
                        true,
                    ));
                    return self.refresh_account_data_for_order_account(&chase_account_address);
                }
                chase.lifecycle = ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrder,
                };
                chase.last_reprice_at = Some(Instant::now());
                self.order_status = Some((
                    format!(
                        "Chase order status ambiguous for oid {oid}: {}; keeping chase state",
                        status.raw_summary
                    ),
                    true,
                ));
                Task::none()
            }
            Ok(status) => {
                let summary = format!(
                    "Chase stopped: orderStatus for oid {oid} was {}",
                    status.raw_summary
                );
                self.fail_chase_order(chase_id, summary);
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
            Err(error) => {
                let error = redact_sensitive_response_text(&error);
                chase.lifecycle = chase.lifecycle.verifying_order_status(oid);
                chase.last_reprice_at = Some(Instant::now());
                self.order_status = Some((
                    format!(
                        concat!(
                            "Chase order status still uncertain for oid {}: {}; ",
                            "keeping chase state"
                        ),
                        oid, error
                    ),
                    true,
                ));
                self.refresh_account_data_for_order_account(&chase_account_address)
            }
        }
    }
}
