mod status;
mod stop_cancel;

use crate::api::fetch_order_status_by_cloid;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::cancel_order_task;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, ChaseVerificationReason, ExchangeResponse};

use iced::Task;
use std::time::Instant;

use super::super::results::result_requires_account_refresh;
#[cfg(test)]
use stop_cancel::StoppedChaseCancelRequest;
use stop_cancel::stopped_chase_cancel_request;

#[cfg(test)]
mod tests;

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

    fn finish_definitive_chase_place_failure(&mut self, chase_id: u64, summary: String) {
        self.clear_chase_startup_pending_if_owned(chase_id);
        let mut summary = summary;
        let mut is_error = true;
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            if chase.lifecycle.is_stopping() {
                (summary, is_error) = chase
                    .stop_reason
                    .clone()
                    .unwrap_or_else(|| ("Chase stopped".to_string(), false));
            }
            chase.desired_price = None;
            chase.stop_reason = Some((summary.clone(), is_error));
        }
        self.order_status = Some((summary.clone(), is_error));
        self.remove_chase_order_with_summary(chase_id, Some(summary));
    }

    pub(crate) fn check_chase_place_status_by_cloid(
        &mut self,
        chase_id: u64,
        reason: String,
    ) -> Task<Message> {
        let chase_account_address = self
            .chase_orders
            .get(&chase_id)
            .map(|chase| chase.account_address.clone());
        let Some((account_address, cloid)) = self.chase_orders.get(&chase_id).and_then(|chase| {
            chase
                .current_cloid
                .as_ref()
                .map(|cloid| (chase.account_address.clone(), cloid.clone()))
        }) else {
            let summary = format!(
                concat!(
                    "Chase placement status unknown: response was not confirmed ({}); ",
                    "no cloid available"
                ),
                reason
            );
            self.fail_chase_order(chase_id, summary);
            return chase_account_address
                .as_deref()
                .map_or_else(Task::none, |address| {
                    self.refresh_account_data_for_order_account(address)
                });
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
                concat!(
                    "Chase placement status unknown: response was not confirmed ({}); ",
                    "checking {}"
                ),
                reason, cloid
            ),
            true,
        ));
        let request_cloid = cloid.clone();
        let account_refresh_task = self.refresh_account_data_for_order_account(&account_address);
        Task::batch([
            account_refresh_task,
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
        let owns_startup_pending = self.chase_place_result_owns_startup_pending(chase_id);
        let should_refresh = result_requires_account_refresh(&result);
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
        if !self
            .chase_orders
            .get(&chase_id)
            .is_some_and(|chase| chase.lifecycle.expects_place_result())
        {
            return self.refresh_after_chase_result_for_order_account(
                should_refresh,
                &chase_account_address,
            );
        }
        if owns_startup_pending {
            self.clear_chase_startup_pending_if_owned(chase_id);
        }

        match result {
            Ok(resp) => {
                if resp.is_error() {
                    self.finish_definitive_chase_place_failure(
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
                    return self.refresh_account_data_for_order_account(&chase_account_address);
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
                        let cancel_task = cancel_order_task(
                            request.agent_key,
                            request.asset,
                            request.oid,
                            move |r| Message::ChaseCancelResult {
                                chase_id: request.chase_id,
                                oid: request.oid,
                                result: Box::new(r),
                            },
                        );
                        return if should_refresh {
                            Task::batch([
                                self.refresh_account_data_for_order_account(&chase_account_address),
                                cancel_task,
                            ])
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
                            format!(
                                "Chase placement accepted (oid {oid}); verifying account state"
                            ),
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
        self.refresh_after_chase_result_for_order_account(should_refresh, &chase_account_address)
    }

    pub(super) fn refresh_after_chase_result_for_order_account(
        &mut self,
        should_refresh: bool,
        account_address: &str,
    ) -> Task<Message> {
        if should_refresh {
            self.refresh_account_data_for_order_account(account_address)
        } else {
            Task::none()
        }
    }
}
