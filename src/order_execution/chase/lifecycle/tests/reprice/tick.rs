use super::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, Duration, Instant, chase,
    chase_by_id, exchange_ready_terminal,
};

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
