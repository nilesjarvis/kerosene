use super::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, Duration, Instant, chase,
    chase_by_id, exchange_ready_terminal,
};
use crate::signing::ChaseStopPhase;

#[test]
fn chase_queue_serves_the_least_recent_request_instead_of_lowest_id() {
    let mut terminal = exchange_ready_terminal();
    let now = Instant::now();
    for (id, age) in [(1, 2), (2, 10), (3, 5)] {
        let mut order = chase();
        order.id = id;
        order.last_reprice_at = Some(now - Duration::from_secs(age));
        order.desired_price = Some(101.0);
        order.lifecycle = ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice,
        };
        terminal.chase_orders.insert(id, order);
    }
    let _task = terminal.handle_chase_reprice_tick();
    assert_eq!(
        chase_by_id(&terminal, 2).lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert!(matches!(
        chase_by_id(&terminal, 1).lifecycle,
        ChaseLifecycle::Queued { .. }
    ));
    assert!(matches!(
        chase_by_id(&terminal, 3).lifecycle,
        ChaseLifecycle::Queued { .. }
    ));
}

#[test]
fn chase_book_event_cannot_overtake_an_older_eligible_queued_order() {
    let mut terminal = exchange_ready_terminal();
    let now = Instant::now();
    let mut incoming = chase();
    incoming.last_reprice_at = Some(now - Duration::from_secs(2));
    terminal.chase_orders.insert(1, incoming);
    let mut waiting = chase();
    waiting.id = 2;
    waiting.last_reprice_at = Some(now - Duration::from_secs(10));
    waiting.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    waiting.desired_price = Some(101.0);
    terminal.chase_orders.insert(2, waiting);
    let _task = terminal.chase_reprice_to_best_price(1, 102.0);
    assert!(!terminal.account_loading);
    assert_eq!(
        chase_by_id(&terminal, 1).lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    let _task = terminal.handle_chase_reprice_tick();
    assert!(chase_by_id(&terminal, 2).needs_account_verification());
}

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
