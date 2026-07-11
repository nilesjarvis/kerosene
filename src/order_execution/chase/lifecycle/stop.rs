use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::positive_finite_value;
use crate::message::Message;
use crate::order_execution::cancel_order_task;
use crate::signing::{ChaseLifecycle, ChaseStopPhase, MAX_CHASE_CANCEL_RETRIES};

use super::{ChaseLimitReason, chase_reprice_limit_reason};

use iced::Task;
use std::time::Instant;

mod planning;

pub(in crate::order_execution::chase::lifecycle) use planning::StopChaseAction;
#[cfg(test)]
pub(in crate::order_execution::chase::lifecycle) use planning::plan_stop_chase;
use planning::plan_stop_chase_with_reason;

// ---------------------------------------------------------------------------
// Stop Execution
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn stop_chase(&mut self) -> Task<Message> {
        let Some(chase_id) = self.selected_chase_id() else {
            return Task::none();
        };
        self.stop_chase_by_id_with_reason(chase_id, "Chase stopped", false)
    }

    pub(crate) fn stop_chase_by_id(&mut self, chase_id: u64) -> Task<Message> {
        self.stop_chase_by_id_with_reason(chase_id, "Chase stopped", false)
    }

    pub(crate) fn stop_chase_by_id_with_reason(
        &mut self,
        chase_id: u64,
        reason: impl Into<String>,
        is_error: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        let clear_startup_pending = self.chase_orders.get(&chase_id).is_some_and(|chase| {
            self.chase_owns_startup_pending_action(chase_id, chase)
                && chase.current_cloid.is_none()
                && !chase.has_pending_op()
        });
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        if clear_startup_pending {
            self.pending_order_action = None;
        }

        let account_address = chase.account_address.clone();
        let market_type = if chase.is_spot {
            MarketType::Spot
        } else {
            MarketType::Perp
        };
        let reason = reason.into();
        match plan_stop_chase_with_reason(chase, reason.clone(), is_error) {
            StopChaseAction::CancelResting {
                chase_id,
                asset,
                oid,
            } => {
                let key = chase.agent_key.clone_for_task();
                if key.is_empty() {
                    self.set_order_status(
                        "Chase stopped: original agent key is unavailable".into(),
                        true,
                    );
                    self.remove_chase_order(chase_id);
                    return Task::none();
                }
                self.set_order_status_toast_on_error(
                    format!("{reason}: cancelling order {oid}"),
                    is_error,
                );
                self.invalidate_spot_balances_after_exchange_dispatch(
                    &account_address,
                    market_type,
                );
                cancel_order_task(key, asset, oid, move |r| Message::ChaseCancelResult {
                    chase_id,
                    oid: oid.into(),
                    result: Box::new(r),
                })
            }
            StopChaseAction::AwaitPlaceResult => {
                self.set_order_status_toast_on_error(
                    format!("{reason}: waiting for order id before cancelling"),
                    is_error,
                );
                Task::none()
            }
            StopChaseAction::AwaitModifyResult => {
                self.set_order_status_toast_on_error(
                    format!("{reason}: modify already in flight"),
                    is_error,
                );
                Task::none()
            }
            StopChaseAction::AwaitCancelResult => {
                self.set_order_status_toast_on_error(
                    format!("{reason}: cancel already in flight"),
                    is_error,
                );
                Task::none()
            }
            StopChaseAction::Clear => {
                self.set_order_status_toast_on_error(reason, is_error);
                self.remove_chase_order(chase_id);
                Task::none()
            }
        }
    }

    pub(crate) fn stop_all_chases(&mut self) -> Task<Message> {
        let ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.stop_chase_by_id_with_reason(id, "Chase stopped", false)),
        )
    }

    pub(super) fn stop_chase_for_limit(
        &mut self,
        chase_id: u64,
        reason: ChaseLimitReason,
    ) -> Task<Message> {
        self.stop_chase_by_id_with_reason(
            chase_id,
            format!("Chase stopped: {}", reason.status_detail()),
            true,
        )
    }

    pub(crate) fn stop_chase_if_limits_reached(&mut self, now: Instant) -> Task<Message> {
        let stops: Vec<_> = self
            .chase_orders
            .iter()
            .filter_map(|(id, chase)| {
                if chase.lifecycle.is_stopping()
                    || positive_finite_value(chase.current_price).is_none()
                {
                    return None;
                }
                chase_reprice_limit_reason(chase, chase.current_price, now)
                    .map(|reason| (*id, reason))
            })
            .collect();
        let tasks = stops
            .into_iter()
            .map(|(id, reason)| self.stop_chase_for_limit(id, reason));
        Task::batch(tasks)
    }

    pub(crate) fn cancel_known_chase_order_for_safety(
        &mut self,
        chase_id: u64,
        oid: u64,
        reason: impl Into<String>,
        is_error: bool,
    ) -> Task<Message> {
        let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
            return Task::none();
        };
        let reason = reason.into();
        if chase.cancel_retries >= MAX_CHASE_CANCEL_RETRIES {
            let manual_status = chase.stop_reason.clone().unwrap_or_else(|| {
                (
                    format!(
                        concat!(
                            "Chase requires manual check: cancel status could not be confirmed ",
                            "after {} attempts; check open orders"
                        ),
                        MAX_CHASE_CANCEL_RETRIES
                    ),
                    true,
                )
            });
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { oid },
            };
            chase.stop_reason = Some(manual_status.clone());
            self.set_order_status_toast_on_error(manual_status.0, manual_status.1);
            return Task::none();
        }
        chase.record_oid(oid);
        chase.current_oid = Some(oid);
        chase.lifecycle = ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid },
        };
        chase.stop_reason = Some((reason.clone(), is_error));
        let key = chase.agent_key.clone_for_task();
        if key.is_empty() {
            self.set_order_status(
                format!("{reason}: manual check required; original agent key is unavailable"),
                true,
            );
            return Task::none();
        }

        let asset = chase.asset;
        let account_address = chase.account_address.clone();
        let market_type = if chase.is_spot {
            MarketType::Spot
        } else {
            MarketType::Perp
        };
        self.set_order_status_toast_on_error(format!("{reason}: cancelling order {oid}"), is_error);
        self.invalidate_spot_balances_after_exchange_dispatch(&account_address, market_type);
        cancel_order_task(key, asset, oid, move |r| Message::ChaseCancelResult {
            chase_id,
            oid: oid.into(),
            result: Box::new(r),
        })
    }

    pub(in crate::order_execution::chase::lifecycle) fn next_stopped_chase_cancel_retry(
        &self,
        now: Instant,
    ) -> Option<(u64, String, bool)> {
        self.chase_orders.iter().find_map(|(id, chase)| {
            if matches!(
                chase.lifecycle,
                ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::VerifyingCancel { .. }
                }
            ) && chase.current_oid.is_some()
                && chase.cancel_retries > 0
                && chase.cancel_retries < crate::signing::MAX_CHASE_CANCEL_RETRIES
                && chase.can_reprice_now(now)
            {
                let (reason, is_error) = chase
                    .stop_reason
                    .clone()
                    .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                Some((*id, reason, is_error))
            } else {
                None
            }
        })
    }

    pub(crate) fn retry_stopped_chase_cancels(&mut self, now: Instant) -> Task<Message> {
        if !self.can_send_chase_exchange_request(now) {
            return Task::none();
        }
        let Some((chase_id, reason, is_error)) = self.next_stopped_chase_cancel_retry(now) else {
            return Task::none();
        };

        self.stop_chase_by_id_with_reason(chase_id, reason, is_error)
    }
}
