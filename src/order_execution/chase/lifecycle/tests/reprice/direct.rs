use super::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, chase, chase_by_id,
    connected_terminal, exchange_busy_terminal, exchange_ready_terminal,
};

#[test]
fn chase_reprice_refreshes_account_before_modifying_resting_order() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert!((chase.remaining_size - 1.0).abs() < 1e-12);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn chase_reprice_updates_desired_price_while_account_verification_is_pending() {
    let mut terminal = connected_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 102.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(102.0));
    assert_eq!(chase.current_price, 100.0);
}

#[test]
fn chase_reprice_queues_when_exchange_gate_is_busy() {
    let mut terminal = exchange_busy_terminal();
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.reprice_count, 0);
}

#[test]
fn chase_reprice_clears_stale_queued_target_when_book_moves_away() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 99.5);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.desired_price, None);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.current_price, 100.0);
}
