use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseStopPhase, ChaseVerificationReason};

// ---------------------------------------------------------------------------
// Stop Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::order_execution::chase::lifecycle) enum StopChaseAction {
    CancelResting { chase_id: u64, asset: u32, oid: u64 },
    AwaitPlaceResult,
    AwaitModifyResult,
    AwaitCancelResult,
    Clear,
}

#[cfg(test)]
pub(in crate::order_execution::chase::lifecycle) fn plan_stop_chase(
    chase: &mut ChaseOrder,
) -> StopChaseAction {
    plan_stop_chase_with_reason(chase, "Chase stopped".to_string(), false)
}

pub(super) fn plan_stop_chase_with_reason(
    chase: &mut ChaseOrder,
    reason: String,
    is_error: bool,
) -> StopChaseAction {
    chase.stop_reason = Some((reason, is_error));
    match chase.lifecycle {
        ChaseLifecycle::Placing => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingPlace,
            };
            StopChaseAction::AwaitPlaceResult
        }
        ChaseLifecycle::Modifying { oid } => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingModify { oid },
            };
            StopChaseAction::AwaitModifyResult
        }
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingPlace,
        } => StopChaseAction::AwaitPlaceResult,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingModify { .. },
        } => StopChaseAction::AwaitModifyResult,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { .. },
        } => StopChaseAction::AwaitCancelResult,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid },
        } => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::Canceling { oid },
            };
            StopChaseAction::CancelResting {
                chase_id: chase.id,
                asset: chase.asset,
                oid,
            }
        }
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Placement,
        } if chase.current_oid.is_none() => {
            chase.lifecycle = ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::AwaitingPlace,
            };
            StopChaseAction::AwaitPlaceResult
        }
        _ => match chase.current_oid {
            Some(oid) => {
                chase.lifecycle = ChaseLifecycle::Stopping {
                    phase: ChaseStopPhase::Canceling { oid },
                };
                StopChaseAction::CancelResting {
                    chase_id: chase.id,
                    asset: chase.asset,
                    oid,
                }
            }
            None => StopChaseAction::Clear,
        },
    }
}
