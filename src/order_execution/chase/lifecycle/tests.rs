use super::{
    ChaseLimitReason, StopChaseAction, chase_account_matches, chase_reprice_limit_reason,
    plan_stop_chase,
};
use crate::signing::{
    ChaseOrder, MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES,
};
use std::time::{Duration, Instant};

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "original-agent-key".to_string().into(),
        is_buy: true,
        remaining_size: 1.0,
        asset: 0,
        sz_decimals: 5,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        initial_price: 100.0,
        started_at,
        reprice_count: 0,
        cancel_in_flight: false,
        stop_requested: false,
        cancel_retries: 0,
        oid_confirmed: true,
    }
}

#[test]
fn stop_chase_waits_for_pending_place_result_before_forgetting_context() {
    let mut chase = chase();
    chase.current_oid = None;

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::AwaitPlaceResult
    );
    assert!(chase.stop_requested);
}

#[test]
fn stop_chase_cancels_resting_order_with_chase_context() {
    let mut chase = chase();

    assert_eq!(
        plan_stop_chase(&mut chase),
        StopChaseAction::CancelResting { asset: 0, oid: 42 }
    );
    assert!(chase.stop_requested);
    assert!(chase.cancel_in_flight);
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
