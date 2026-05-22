use crate::api::{OrderStatusResult, fetch_order_status_by_cloid};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{
    ChaseLifecycle, ChaseOrder, ChaseStopPhase, ChaseVerificationReason, ExchangeResponse,
    cancel_order,
};

use iced::Task;
use std::time::Instant;
use zeroize::Zeroizing;

use super::super::results::result_requires_account_refresh;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoppedChaseCancelRequest {
    pub(super) chase_id: u64,
    pub(super) agent_key: Zeroizing<String>,
    pub(super) asset: u32,
    pub(super) oid: u64,
}

pub(super) fn stopped_chase_cancel_request(
    chase: &ChaseOrder,
    response: &ExchangeResponse,
) -> Option<StoppedChaseCancelRequest> {
    if !chase.lifecycle.is_stopping() || response.is_error() || response.is_fully_filled() {
        return None;
    }
    let agent_key = chase.agent_key.trim();
    if agent_key.is_empty() {
        return None;
    }
    Some(StoppedChaseCancelRequest {
        chase_id: chase.id,
        agent_key: agent_key.to_string().into(),
        asset: chase.asset,
        oid: response.order_oid()?,
    })
}

impl TradingTerminal {
    fn fail_chase_order(&mut self, chase_id: u64, summary: String) {
        let mut keep_for_verification = false;
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            keep_for_verification = chase.has_exchange_identifier();
            chase.lifecycle = if chase.current_oid.is_none() && chase.current_cloid.is_some() {
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Placement,
                }
            } else {
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrder,
                }
            };
            chase.stop_reason = Some((summary.clone(), true));
        }
        self.order_status = Some((summary.clone(), true));
        if !keep_for_verification {
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
    }

    pub(crate) fn check_chase_place_status_by_cloid(
        &mut self,
        chase_id: u64,
        reason: String,
    ) -> Task<Message> {
        let Some((account_address, cloid)) = self.chase_orders.get(&chase_id).and_then(|chase| {
            chase
                .current_cloid
                .as_ref()
                .map(|cloid| (chase.account_address.clone(), cloid.clone()))
        }) else {
            let summary = format!(
                "Chase placement status unknown: response was not confirmed ({reason}); no cloid available"
            );
            self.fail_chase_order(chase_id, summary);
            return self.refresh_account_data();
        };

        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            if !chase.lifecycle.is_stopping() {
                chase.lifecycle = ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Placement,
                };
            }
            chase.last_reprice_at = Some(Instant::now());
        }
        self.order_status = Some((
            format!(
                "Chase placement status unknown: response was not confirmed ({reason}); checking {cloid}"
            ),
            true,
        ));
        let request_cloid = cloid.clone();
        Task::batch([
            self.refresh_account_data(),
            Task::perform(
                fetch_order_status_by_cloid(account_address, request_cloid),
                move |result| Message::ChaseOrderStatusLoaded {
                    chase_id,
                    cloid,
                    result: Box::new(result),
                },
            ),
        ])
    }

    pub(crate) fn handle_chase_place_result(
        &mut self,
        chase_id: u64,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        let should_refresh = result_requires_account_refresh(&result);
        if !self.chase_orders.contains_key(&chase_id) {
            return self.refresh_after_chase_result(should_refresh);
        }
        if !self
            .chase_orders
            .get(&chase_id)
            .is_some_and(|chase| chase.lifecycle.expects_place_result())
        {
            return self.refresh_after_chase_result(should_refresh);
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    self.fail_chase_order(
                        chase_id,
                        format!("Chase place failed: {}", resp.summary()),
                    );
                } else if resp.is_fully_filled() {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        if let Some(oid) = resp.order_oid() {
                            chase.record_oid(oid);
                        }
                        let filled_size = resp.filled_total_size().unwrap_or(chase.remaining_size);
                        chase.add_filled_size(filled_size);
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder,
                        };
                    }
                    self.order_status = Some((resp.summary(), false));
                    return self.refresh_account_data();
                } else {
                    let stop_cancel_request = self
                        .chase_orders
                        .get(&chase_id)
                        .and_then(|chase| stopped_chase_cancel_request(chase, &resp));
                    if let Some(request) = stop_cancel_request {
                        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                            chase.current_oid = Some(request.oid);
                            chase.lifecycle = ChaseLifecycle::Stopping {
                                phase: ChaseStopPhase::Canceling { oid: request.oid },
                            };
                        }
                        self.order_status = Some((
                            format!("Chase stopping: cancelling placed order {}", request.oid),
                            false,
                        ));
                        let cancel_task = Task::perform(
                            cancel_order(request.agent_key, request.asset, request.oid),
                            move |r| Message::ChaseCancelResult {
                                chase_id: request.chase_id,
                                oid: request.oid,
                                result: Box::new(r),
                            },
                        );
                        return if should_refresh {
                            Task::batch([self.refresh_account_data(), cancel_task])
                        } else {
                            cancel_task
                        };
                    }

                    if let Some(oid) = resp.order_oid() {
                        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                            chase.record_oid(oid);
                            chase.current_oid = Some(oid);
                            chase.lifecycle = ChaseLifecycle::Verifying {
                                reason: ChaseVerificationReason::Placement,
                            };
                            chase.cancel_retries = 0;
                        }
                        self.order_status = Some((
                            format!("Chase placement accepted (oid {oid}); verifying account state"),
                            false,
                        ));
                    } else {
                        return self.check_chase_place_status_by_cloid(chase_id, resp.summary());
                    }
                }
            }
            Err(e) => {
                return self.check_chase_place_status_by_cloid(chase_id, e);
            }
        }
        self.refresh_after_chase_result(should_refresh)
    }

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

        match result {
            Ok(status) if status.is_open() => {
                let Some(oid) = status.oid else {
                    let summary = format!(
                        "Chase stopped: placement status open for {cloid} but no order id was returned"
                    );
                    self.fail_chase_order(chase_id, summary);
                    return self.refresh_account_data();
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
                self.refresh_account_data()
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
                self.refresh_account_data()
            }
            Ok(status) if status.is_definitive_no_fill_terminal() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::MissingOrderResolvedNoFill,
                    };
                }
                self.order_status = Some((
                    format!(
                        "Chase checking account state: placement resolved without fill as {}",
                        status.raw_summary
                    ),
                    false,
                ));
                self.refresh_account_data()
            }
            Ok(status) if status.is_no_fill_terminal() => {
                let summary = format!(
                    "Chase stopped: placement is no longer open ({}); waiting for account reconciliation",
                    status.raw_summary
                );
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    if let Some(oid) = status.oid {
                        chase.record_oid(oid);
                        chase.current_oid = Some(oid);
                        chase.lifecycle = ChaseLifecycle::Stopping {
                            phase: ChaseStopPhase::VerifyingCancel { oid },
                        };
                    } else {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::Placement,
                        };
                    }
                    chase.desired_price = None;
                    chase.stop_reason = Some((summary.clone(), true));
                    chase.last_reprice_at = Some(Instant::now());
                }
                self.order_status = Some((summary, true));
                self.refresh_account_data()
            }
            Ok(status) if status.is_missing() => {
                if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                    chase.lifecycle = ChaseLifecycle::Verifying {
                        reason: ChaseVerificationReason::Placement,
                    };
                    chase.last_reprice_at = Some(Instant::now());
                }
                self.order_status = Some((
                    format!(
                        "Chase placement status ambiguous for {cloid}: {}; keeping chase state",
                        status.raw_summary
                    ),
                    true,
                ));
                self.refresh_account_data()
            }
            Ok(status) => {
                let summary = format!(
                    "Chase stopped: placement status for {cloid} was {}",
                    status.raw_summary
                );
                self.fail_chase_order(chase_id, summary);
                self.refresh_account_data()
            }
            Err(error) => {
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
                        "Chase placement status still uncertain for {cloid}: {error}; keeping chase state"
                    ),
                    true,
                ));
                self.refresh_account_data()
            }
        }
    }

    pub(super) fn refresh_after_chase_result(&mut self, should_refresh: bool) -> Task<Message> {
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }

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
                            "Chase stop status ambiguous for oid {oid}: {}; verifying account state",
                            status.raw_summary
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
                        "Chase order status still uncertain for oid {oid}: {error}; keeping chase state"
                    ),
                    true,
                ));
                self.refresh_account_data()
            }
        }
    }
}
