use crate::api::{OrderStatusResult, fetch_order_status_by_cloid};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseOrder, ChasePendingOp, ExchangeResponse, cancel_order};

use iced::Task;
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
    if !chase.stop_requested || response.is_error() || response.is_fully_filled() {
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
        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
            chase.pending_op = None;
            chase.stop_requested = true;
            chase.stop_reason = Some((summary.clone(), true));
        }
        self.order_status = Some((summary.clone(), true));
        self.remove_chase_order_with_summary(chase_id, Some(summary));
    }

    fn check_chase_place_status_by_cloid(
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
            .is_some_and(|chase| matches!(chase.pending_op, Some(ChasePendingOp::Place)))
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
                    }
                    self.order_status = Some((resp.summary(), false));
                    self.remove_chase_order(chase_id);
                } else {
                    let stop_cancel_request = self
                        .chase_orders
                        .get(&chase_id)
                        .and_then(|chase| stopped_chase_cancel_request(chase, &resp));
                    if let Some(request) = stop_cancel_request {
                        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                            chase.current_oid = Some(request.oid);
                            chase.pending_op = Some(ChasePendingOp::Cancel { oid: request.oid });
                            chase.pending_size_correction = false;
                            chase.oid_confirmed = false;
                            chase.missing_open_order_refresh_requested = false;
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
                            chase.pending_op = None;
                            chase.pending_size_correction = false;
                            chase.oid_confirmed = false;
                            chase.cancel_retries = 0;
                            chase.missing_open_order_refresh_requested = false;
                        }
                        self.order_status = Some((format!("Chasing (oid {oid})..."), false));
                    } else {
                        self.order_status = Some((resp.summary(), false));
                        self.remove_chase_order(chase_id);
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
        if !matches!(chase.pending_op, Some(ChasePendingOp::Place))
            || chase.current_cloid.as_deref() != Some(cloid.as_str())
        {
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
                    chase.record_oid(oid);
                    chase.current_oid = Some(oid);
                    chase.pending_op = None;
                    chase.pending_size_correction = false;
                    chase.oid_confirmed = false;
                    chase.cancel_retries = 0;
                    chase.missing_open_order_refresh_requested = true;
                    chase.stop_requested.then(|| {
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
                }
                let summary = format!(
                    "Chase placement filled according to orderStatus: {}; refreshing account data",
                    status.raw_summary
                );
                self.order_status = Some((summary.clone(), false));
                self.remove_chase_order_with_summary(chase_id, Some(summary));
                self.refresh_account_data()
            }
            Ok(status) if status.is_missing() || status.is_no_fill_terminal() => {
                let summary = format!(
                    "Chase stopped: placement resolved as {}",
                    status.raw_summary
                );
                self.fail_chase_order(chase_id, summary);
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
                let summary = format!(
                    "Chase stopped: could not confirm placement status for {cloid}: {error}"
                );
                self.fail_chase_order(chase_id, summary);
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
}
