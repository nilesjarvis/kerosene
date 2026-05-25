use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseQueuedAction, ChaseStopPhase, ChaseVerificationReason};

use iced::Task;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Chase Reprice Tick
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChaseStatusRetry {
    Placement,
    Oid { oid: u64 },
}

impl TradingTerminal {
    pub(crate) fn handle_chase_reprice_tick(&mut self) -> Task<Message> {
        let now = Instant::now();
        if !self.can_send_chase_exchange_request(now) {
            return Task::none();
        }

        if let Some((chase_id, retry)) = self.next_chase_status_retry(now) {
            return match retry {
                ChaseStatusRetry::Placement => self.check_chase_place_status_by_cloid(
                    chase_id,
                    "retrying placement status".to_string(),
                ),
                ChaseStatusRetry::Oid { oid } => self.check_chase_order_status(
                    chase_id,
                    oid,
                    "Chase retrying order status check",
                ),
            };
        }

        let Some((chase_id, action)) = self.next_queued_chase_reprice_action(now) else {
            return Task::none();
        };
        match action {
            ChaseQueuedAction::Place => {
                let Some(best) = self
                    .chase_orders
                    .get(&chase_id)
                    .and_then(|chase| chase.desired_price)
                else {
                    return Task::none();
                };
                self.chase_place_at_best(chase_id, best)
            }
            ChaseQueuedAction::Reprice => {
                let Some(best) = self
                    .chase_orders
                    .get(&chase_id)
                    .and_then(|chase| chase.desired_price)
                else {
                    return Task::none();
                };
                self.chase_reprice_to_best_price(chase_id, best)
            }
            ChaseQueuedAction::SizeCorrection => {
                self.chase_modify_for_current_price_reconciliation(chase_id)
            }
        }
    }

    fn next_chase_status_retry(&self, now: Instant) -> Option<(u64, ChaseStatusRetry)> {
        self.chase_orders.iter().find_map(|(id, chase)| {
            if !chase.can_reprice_now(now) {
                return None;
            }
            match chase.lifecycle {
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Placement,
                }
                | ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::AwaitingPlace,
                } if chase.current_oid.is_none() && chase.current_cloid.is_some() => {
                    Some((*id, ChaseStatusRetry::Placement))
                }
                ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::Modify,
                }
                | ChaseLifecycle::Verifying {
                    reason: ChaseVerificationReason::MissingOrder,
                } => chase
                    .current_oid
                    .map(|oid| (*id, ChaseStatusRetry::Oid { oid })),
                _ => None,
            }
        })
    }

    fn next_queued_chase_reprice_action(&self, now: Instant) -> Option<(u64, ChaseQueuedAction)> {
        self.chase_orders.iter().find_map(|(id, chase)| {
            if chase.has_pending_op() {
                return None;
            }
            if !chase.can_reprice_now(now) {
                return None;
            }
            match chase.lifecycle {
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Place,
                } if chase.desired_price.is_some() => Some((*id, ChaseQueuedAction::Place)),
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::Reprice,
                } if chase.desired_price.is_some() => Some((*id, ChaseQueuedAction::Reprice)),
                ChaseLifecycle::Queued {
                    action: ChaseQueuedAction::SizeCorrection,
                } => Some((*id, ChaseQueuedAction::SizeCorrection)),
                _ => None,
            }
        })
    }
}
