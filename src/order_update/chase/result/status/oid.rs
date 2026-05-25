use crate::api::OrderStatusResult;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason};

use iced::Task;
use std::time::Instant;

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
        let Some(chase) = self.chase_orders.get(&chase_id) else {
            return Task::none();
        };
        if chase.current_oid != Some(oid) {
            return Task::none();
        }

        match result {
            Ok(status) if status.is_open() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
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
                }
                if self
                    .chase_orders
                    .get(&chase_id)
                    .is_some_and(|chase| chase.lifecycle.is_stopping())
                {
                    let (reason, is_error) = self
                        .chase_orders
                        .get(&chase_id)
                        .and_then(|chase| chase.stop_reason.clone())
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
                self.refresh_account_data()
            }
            Ok(status) if status.is_filled() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.record_oid(oid);
                    let filled_size = chase.remaining_size;
                    chase.add_filled_size(filled_size);
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                }
                let summary = format!(
                    "Chase order filled according to orderStatus: {}; refreshing account data",
                    status.raw_summary
                );
                self.order_status = Some((summary.clone(), false));
                self.refresh_account_data()
            }
            Ok(status) if status.is_definitive_no_fill_terminal() => {
                if self
                    .chase_orders
                    .get(&chase_id)
                    .is_some_and(|chase| chase.lifecycle.is_stopping())
                {
                    let (message, is_error) = self
                        .chase_orders
                        .get(&chase_id)
                        .and_then(|chase| chase.stop_reason.clone())
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    self.order_status = Some((message.clone(), is_error));
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        chase.current_oid = Some(oid);
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { oid },
                        };
                    }
                    return self.refresh_account_data();
                }
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrderResolvedNoFill,
                    };
                }
                self.order_status = Some((
                    format!(
                        "Chase checking account state: orderStatus resolved without fill as {}",
                        status.raw_summary
                    ),
                    false,
                ));
                self.refresh_account_data()
            }
            Ok(status) if status.is_no_fill_terminal() => {
                let summary = format!(
                    "Chase stopped: order no longer open ({}); no replacement will be placed",
                    status.raw_summary
                );
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.record_oid(oid);
                    chase.current_oid = Some(oid);
                    chase.desired_price = None;
                    chase.stop_reason = Some((summary.clone(), true));
                    chase.lifecycle = ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::VerifyingCancel { oid },
                    };
                }
                self.order_status = Some((summary, true));
                self.refresh_account_data()
            }
            Ok(status) if status.is_missing() => {
                if self
                    .chase_orders
                    .get(&chase_id)
                    .is_some_and(|chase| chase.lifecycle.is_stopping())
                {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        chase.current_oid = Some(oid);
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { oid },
                        };
                        chase.last_reprice_at = Some(Instant::now());
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
                    return self.refresh_account_data();
                }
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrder,
                    };
                    chase.last_reprice_at = Some(Instant::now());
                }
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
                self.refresh_account_data()
            }
            Err(error) => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    if chase.lifecycle.is_stopping() {
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { oid },
                        };
                    } else if matches!(
                        chase.lifecycle,
                        ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder
                                | ChaseVerificationReason::MissingOrderResolvedNoFill
                        }
                    ) {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder,
                        };
                    } else {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::Modify,
                        };
                    }
                    chase.last_reprice_at = Some(Instant::now());
                }
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
                self.refresh_account_data()
            }
        }
    }
}
