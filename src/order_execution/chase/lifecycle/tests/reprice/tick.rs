use super::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, Duration, Instant, chase,
    chase_by_id, exchange_ready_terminal,
};
use crate::signing::ChaseStopPhase;

#[test]
fn chase_reprice_tick_runs_queued_size_correction() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::SizeCorrection,
    };
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_reprice_tick();

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert_eq!(chase.desired_price, Some(100.0));
}

#[test]
fn queued_size_correction_waits_during_pending_exit_and_resumes_if_exit_aborts() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::SizeCorrection,
    };
    terminal.chase_orders.insert(1, chase);
    terminal.config_save_exit_requested = true;

    let blocked_task = terminal.handle_chase_reprice_tick();

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(blocked_task.units(), 0);
    assert_eq!(chase.reprice_count, 0);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::SizeCorrection
        }
    );

    terminal.config_save_exit_requested = false;
    let resumed_task = terminal.handle_chase_reprice_tick();

    assert_eq!(resumed_task.units(), 1);
    assert_eq!(
        chase_by_id(&terminal, 1).lifecycle,
        ChaseLifecycle::Modifying { oid: 42 }
    );
}

#[test]
fn missing_order_verification_is_retried_by_tick() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    chase.last_reprice_at = Some(Instant::now() - Duration::from_secs(10));
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_reprice_tick();

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, _is_error)| {
                message.contains("retrying order status check")
            })
    );
}

#[test]
fn stopped_chase_cancel_retry_is_rearmed_by_tick() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.cancel_retries = 1;
    chase.last_reprice_at = Some(Instant::now() - Duration::from_secs(2));
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_reprice_tick();

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, _is_error)| message.contains("cancelling order 42"))
    );
}

#[test]
fn pending_exit_preserves_exposure_reducing_chase_cancel_retry() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.cancel_retries = 1;
    chase.last_reprice_at = Some(Instant::now() - Duration::from_secs(2));
    terminal.chase_orders.insert(1, chase);
    terminal.config_save_exit_requested = true;

    let task = terminal.handle_chase_reprice_tick();

    assert_eq!(task.units(), 1);
    assert_eq!(
        chase_by_id(&terminal, 1).lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}
