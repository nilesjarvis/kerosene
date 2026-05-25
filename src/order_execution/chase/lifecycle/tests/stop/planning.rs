use super::{ChaseLifecycle, ChaseStopPhase, StopChaseAction, chase, plan_stop_chase};

#[test]
fn stop_chase_waits_for_pending_place_result_before_forgetting_context() {
    let mut chase = chase();
    chase.current_oid = None;
    chase.lifecycle = ChaseLifecycle::Placing;

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitPlaceResult
    );
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingPlace
        }
    );
    assert_eq!(
        chase.stop_reason,
        Some(("Chase stopped".to_string(), false))
    );
}

#[test]
fn stop_chase_cancels_resting_order_with_chase_context() {
    let mut chase = chase();

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::CancelResting {
            chase_id: 1,
            asset: 0,
            oid: 42
        }
    );
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}

#[test]
fn stop_chase_waits_for_pending_modify() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Modifying { oid: 42 };

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitModifyResult
    );
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::AwaitingModify { oid: 42 }
        }
    );
}

#[test]
fn stop_chase_waits_for_pending_cancel_result() {
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::Canceling { oid: 42 },
    };

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitCancelResult
    );
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}
