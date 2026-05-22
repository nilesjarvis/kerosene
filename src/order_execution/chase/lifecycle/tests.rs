use super::{
    ChaseLimitReason, StopChaseAction, chase_account_matches, chase_reprice_limit_reason,
    plan_stop_chase,
};
use crate::app_state::TradingTerminal;
use crate::signing::{
    ChaseLifecycle, ChaseOrder, ChaseQueuedAction, ChaseStopPhase, ChaseVerificationReason,
    MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES,
};
use std::time::{Duration, Instant};

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "original-agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at,
        started_at_ms: 1_000,
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

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

#[test]
fn retry_stopped_chase_cancels_rearms_retryable_cancel_failure() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.cancel_retries = 1;
    chase.last_reprice_at = Some(Instant::now() - Duration::from_secs(2));
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.retry_stopped_chase_cancels(Instant::now());

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}

#[test]
fn chase_context_allows_same_connected_account() {
    assert!(chase_account_matches(
        &chase(),
        Some("0xabc0000000000000000000000000000000000000")
    ));
}

#[test]
fn chase_context_rejects_changed_or_disconnected_account() {
    assert!(!chase_account_matches(
        &chase(),
        Some("0xdef0000000000000000000000000000000000000")
    ));
    assert!(!chase_account_matches(&chase(), None));
}

#[test]
fn chase_exchange_requests_pause_while_account_reconciliation_is_loading() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;

    assert!(terminal.can_send_chase_exchange_request(now));

    terminal.account_loading = true;

    assert!(!terminal.can_send_chase_exchange_request(now));

    terminal.account_loading = false;
    terminal.account_reconciliation_required = true;

    assert!(!terminal.can_send_chase_exchange_request(now));
}

#[test]
fn chase_reprice_refreshes_account_before_modifying_resting_order() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
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
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 102.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.lifecycle, ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::Reprice
    });
    assert_eq!(chase.desired_price, Some(102.0));
    assert_eq!(chase.current_price, 100.0);
}

#[test]
fn chase_reprice_queues_when_exchange_gate_is_busy() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = Some(Instant::now());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
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
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::Reprice,
    };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 99.5);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.desired_price, None);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(chase.current_price, 100.0);
}

#[test]
fn chase_reconciliation_uses_pending_target_price() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.current_price = f64::NAN;
    chase.current_price_wire.clear();
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert_eq!(chase.desired_price, Some(101.0));
}

#[test]
fn chase_reconciliation_queues_size_correction_when_exchange_gate_is_busy() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = Some(Instant::now());
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::SizeCorrection
        }
    );
}

#[test]
fn chase_reprice_tick_runs_queued_size_correction() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Queued {
        action: ChaseQueuedAction::SizeCorrection,
    };
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_reprice_tick();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert_eq!(chase.desired_price, Some(100.0));
}

#[test]
fn chase_place_uses_unfilled_residual_size() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert!((chase.remaining_size - 0.9).abs() < 1e-12);
}

#[test]
fn chase_place_assigns_unique_cloid_per_place_attempt() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.current_oid = None;
    chase.known_oids.clear();
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.lifecycle, ChaseLifecycle::Placing);
    assert_eq!(chase.place_attempt_count, 1);
    assert!(
        chase
            .current_cloid
            .as_deref()
            .is_some_and(|cloid| { cloid.starts_with("0x") && cloid.len() == 34 })
    );
}

#[test]
fn missing_order_verification_is_retried_by_tick() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Verifying {
        reason: ChaseVerificationReason::MissingOrder,
    };
    chase.last_reprice_at = Some(Instant::now() - Duration::from_secs(10));
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_reprice_tick();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
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
fn chase_reprice_limits_allow_normal_price_updates() {
    let chase = chase();

    assert_eq!(
        chase_reprice_limit_reason(
            &chase,
            100.0 * (1.0 + MAX_CHASE_DRIFT_FRACTION),
            Instant::now()
        ),
        None
    );
}

#[test]
fn chase_reprice_limits_use_longer_hard_stops() {
    assert_eq!(MAX_CHASE_DURATION, Duration::from_secs(15 * 60));
    assert_eq!(MAX_CHASE_REPRICES, 1_000);
    assert!((MAX_CHASE_DRIFT_FRACTION - 0.05).abs() < f64::EPSILON);
}

#[test]
fn chase_reprice_limits_stop_invalid_prices() {
    let chase = chase();

    assert_eq!(
        chase_reprice_limit_reason(&chase, f64::INFINITY, Instant::now()),
        Some(ChaseLimitReason::InvalidPrice)
    );
}

#[test]
fn chase_reprice_limits_stop_after_timeout() {
    let mut chase = chase();
    let now = chase.started_at + MAX_CHASE_DURATION + Duration::from_secs(1);

    assert_eq!(
        chase_reprice_limit_reason(&chase, 100.0, now),
        Some(ChaseLimitReason::Timeout {
            elapsed: MAX_CHASE_DURATION + Duration::from_secs(1)
        })
    );

    chase.started_at = now;
    assert_eq!(chase_reprice_limit_reason(&chase, 100.0, now), None);
}

#[test]
fn chase_reprice_limits_stop_at_max_reprices() {
    let mut chase = chase();
    chase.reprice_count = MAX_CHASE_REPRICES;

    assert_eq!(
        chase_reprice_limit_reason(&chase, 100.0, Instant::now()),
        Some(ChaseLimitReason::MaxReprices {
            count: MAX_CHASE_REPRICES
        })
    );
}

#[test]
fn chase_reprice_limits_stop_after_drift_limit() {
    let chase = chase();
    let next_price = 100.0 * (1.0 + MAX_CHASE_DRIFT_FRACTION + 0.001);

    assert_eq!(
        chase_reprice_limit_reason(&chase, next_price, Instant::now()),
        Some(ChaseLimitReason::Drift {
            drift_fraction: (next_price - 100.0) / 100.0
        })
    );
}
