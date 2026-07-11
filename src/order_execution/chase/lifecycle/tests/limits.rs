use super::super::{ChaseLimitReason, chase_reprice_limit_reason};
use super::chase;
use crate::signing::{MAX_CHASE_DRIFT_FRACTION, MAX_CHASE_DURATION, MAX_CHASE_REPRICES};

use std::time::{Duration, Instant};

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

#[test]
fn chase_limit_debug_redacts_drift_without_changing_it() {
    const DRIFT_FRACTION: f64 = 0.061_234_567_89;
    let reason = ChaseLimitReason::Drift {
        drift_fraction: DRIFT_FRACTION,
    };

    let rendered = format!("{reason:?}");

    assert!(rendered.contains("Drift"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(
        !rendered.contains(&format!("{DRIFT_FRACTION:?}")),
        "{rendered}"
    );
    let ChaseLimitReason::Drift { drift_fraction } = reason else {
        panic!("expected drift reason");
    };
    assert_eq!(drift_fraction.to_bits(), DRIFT_FRACTION.to_bits());
}
