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
                    self.order_status =
                        Some((format!("Chase place failed: {}", resp.summary()), true));
                    self.remove_chase_order(chase_id);
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
                self.order_status = Some((
                    format!(
                        "Chase placement status unknown: response was not confirmed ({e}); refreshing account data"
                    ),
                    true,
                ));
                self.remove_chase_order(chase_id);
                return self.refresh_account_data();
            }
        }
        self.refresh_after_chase_result(should_refresh)
    }

    pub(super) fn refresh_after_chase_result(&mut self, should_refresh: bool) -> Task<Message> {
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }
}
