use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseOrder, ExchangeResponse, cancel_order};

use iced::Task;
use zeroize::Zeroizing;

use super::super::results::result_requires_account_refresh;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoppedChaseCancelRequest {
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
        agent_key: agent_key.to_string().into(),
        asset: chase.asset,
        oid: response.order_oid()?,
    })
}

impl TradingTerminal {
    pub(crate) fn handle_chase_place_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        let should_refresh = result_requires_account_refresh(&result);
        match result {
            Ok(resp) => {
                let stop_cancel_request = self
                    .active_chase
                    .as_ref()
                    .and_then(|chase| stopped_chase_cancel_request(chase, &resp));
                if let Some(request) = stop_cancel_request {
                    if let Some(chase) = &mut self.active_chase {
                        chase.current_oid = Some(request.oid);
                        chase.cancel_in_flight = true;
                        chase.oid_confirmed = false;
                    }
                    self.order_status = Some((
                        format!("Chase stopping: cancelling placed order {}", request.oid),
                        false,
                    ));
                    let cancel_task = Task::perform(
                        cancel_order(request.agent_key, request.asset, request.oid),
                        |r| Message::ChaseCancelResult(Box::new(r)),
                    );
                    return if should_refresh {
                        Task::batch([self.refresh_account_data(), cancel_task])
                    } else {
                        cancel_task
                    };
                }

                if resp.is_error() {
                    self.order_status = Some((resp.summary(), true));
                    self.active_chase = None;
                } else if resp.is_fully_filled() {
                    self.order_status = Some((resp.summary(), false));
                    self.active_chase = None;
                } else if let Some(oid) = resp.order_oid() {
                    if let Some(chase) = &mut self.active_chase {
                        if chase.stop_requested {
                            self.order_status = Some((
                                format!(
                                    "Chase stopped but could not cancel placed order {oid}: original agent key is unavailable"
                                ),
                                true,
                            ));
                            self.active_chase = None;
                            return if should_refresh {
                                self.refresh_account_data()
                            } else {
                                Task::none()
                            };
                        }
                        chase.current_oid = Some(oid);
                        chase.cancel_in_flight = false;
                        chase.oid_confirmed = false;
                        self.order_status = Some((format!("Chasing (oid {oid})..."), false));
                    } else {
                        self.order_status = Some((
                            format!(
                                "Chase order placed after chase ended; check open orders (oid {oid})"
                            ),
                            true,
                        ));
                    }
                } else {
                    self.order_status = Some((resp.summary(), false));
                    self.active_chase = None;
                }
            }
            Err(e) => {
                self.order_status = Some((e, true));
                self.active_chase = None;
            }
        }
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }
}
