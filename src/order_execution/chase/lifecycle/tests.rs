use super::{
    ChaseLimitReason, StopChaseAction, chase_account_matches, chase_reprice_limit_reason,
    plan_stop_chase,
};
use crate::app_state::TradingTerminal;
use crate::signing::{
    ChaseOrder, ChasePendingOp, MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES,
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
        adopted_resting: false,
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        pending_size_correction: false,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: true,
        missing_open_order_refresh_requested: false,
    }
}

#[test]
fn stop_chase_waits_for_pending_place_result_before_forgetting_context() {
    let mut chase = chase();
    chase.current_oid = None;
    chase.pending_op = Some(ChasePendingOp::Place);

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitPlaceResult
    );
    assert!(chase.stop_requested);
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
    assert!(chase.stop_requested);
    assert_eq!(chase.pending_op, Some(ChasePendingOp::Cancel { oid: 42 }));
}

#[test]
fn stop_chase_waits_for_pending_modify() {
    let mut chase = chase();
    chase.pending_op = Some(ChasePendingOp::Modify { oid: 42 });

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitModifyResult
    );
    assert!(chase.stop_requested);
    assert_eq!(chase.pending_op, Some(ChasePendingOp::Modify { oid: 42 }));
}

#[test]
fn stop_chase_waits_for_pending_cancel_result() {
    let mut chase = chase();
    chase.pending_op = Some(ChasePendingOp::Cancel { oid: 42 });

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitCancelResult
    );
    assert!(chase.stop_requested);
    assert_eq!(chase.pending_op, Some(ChasePendingOp::Cancel { oid: 42 }));
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
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.pending_best_price, Some(101.0));
    assert!(chase.missing_open_order_refresh_requested);
    assert!((chase.remaining_size - 1.0).abs() < 1e-12);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn chase_reprice_waits_for_open_order_confirmation() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.oid_confirmed = false;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_reprice_to_best_price(1, 101.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.pending_best_price, Some(101.0));
    assert_eq!(chase.reprice_count, 0);
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
    chase.pending_best_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, Some(ChasePendingOp::Modify { oid: 42 }));
    assert_eq!(chase.pending_best_price, Some(101.0));
}

#[test]
fn chase_reconciliation_queues_size_correction_when_exchange_gate_is_busy() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = Some(Instant::now());
    let mut chase = chase();
    chase.pending_size_correction = true;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert!(chase.pending_size_correction);
}

#[test]
fn chase_reprice_tick_runs_queued_size_correction() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.pending_size_correction = true;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_reprice_tick();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, Some(ChasePendingOp::Modify { oid: 42 }));
    assert!(chase.pending_size_correction);
    assert_eq!(chase.pending_best_price, Some(100.0));
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
    chase.filled_size = 0.1;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_place_at_best(1, 101.0);

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, Some(ChasePendingOp::Place));
    assert!((chase.remaining_size - 0.9).abs() < 1e-12);
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
