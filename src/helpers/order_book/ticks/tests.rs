use super::*;
use proptest::prelude::*;

#[test]
fn book_tick_validation_accepts_only_positive_finite_values() {
    assert!(valid_book_tick_size(0.01));
    assert!(!valid_book_tick_size(0.0));
    assert!(!valid_book_tick_size(-0.01));
    assert!(!valid_book_tick_size(f64::NAN));
    assert!(!valid_book_tick_size(f64::INFINITY));
}

#[test]
fn tick_helpers_fallback_for_invalid_prices_or_ticks() {
    assert_eq!(default_tick_for_price(f64::NAN), 0.01);
    assert_eq!(default_tick_for_price(f64::INFINITY), 0.01);
    assert_eq!(compute_sigfigs(f64::NAN, 100.0), (None, None));
    assert_eq!(compute_sigfigs(0.01, f64::NAN), (None, None));
    assert_eq!(sigfig_server_tick((None, None), 100.0), None);
    assert_eq!(sigfig_server_tick((Some(5), None), f64::NAN), None);
    assert_eq!(format_tick(f64::NAN), "-");
}

#[test]
fn sigfig_server_tick_reconstructs_exchange_precision() {
    assert_eq!(sigfig_server_tick((Some(5), None), 80_000.0), Some(1.0));
    assert_eq!(sigfig_server_tick((Some(5), Some(5)), 80_000.0), Some(5.0));
    assert_eq!(sigfig_server_tick((Some(4), None), 80_000.0), Some(10.0));
}

#[test]
fn tick_size_matching_allows_tiny_float_drift_only() {
    assert!(tick_sizes_match(1.0, 1.004));
    assert!(!tick_sizes_match(1.0, 1.02));
    assert!(!tick_sizes_match(0.0, 1.0));
}

proptest! {
    /// `default_tick_for_price` should always return a clean power-of-10
    /// tick (i.e. its base-10 log is an integer) for any positive finite mid.
    #[test]
    fn default_tick_for_price_returns_clean_power_of_ten(mid in 1e-6f64..1e9f64) {
        let tick = default_tick_for_price(mid);
        prop_assert!(tick.is_finite() && tick > 0.0);
        let log_diff = (tick.log10() - tick.log10().round()).abs();
        prop_assert!(log_diff < 1e-9, "tick {tick} is not a clean power of 10");
    }

    /// `default_tick_for_price` aims for ~0.01% of the mid (then rounds to a
    /// power of 10). The chosen tick should be at most the raw target and at
    /// least one decade below it.
    #[test]
    fn default_tick_for_price_stays_within_two_decades_of_target(mid in 1e-2f64..1e8f64) {
        let tick = default_tick_for_price(mid);
        let raw_target = mid * 1e-4;
        prop_assert!(tick <= raw_target * 1.0001);
        prop_assert!(tick >= raw_target * 0.0999, "tick {tick} too coarse vs target {raw_target}");
    }

    /// `tick_decimals` is the number of fractional digits needed to express
    /// the tick. For any clean power-of-10 tick <= 1, that's `-log10(tick)`.
    #[test]
    fn tick_decimals_matches_log10_for_power_of_ten_ticks(exp in -8i32..=0i32) {
        let tick = 10f64.powi(exp);
        let expected = (-exp).max(0) as usize;
        prop_assert_eq!(tick_decimals(tick), expected);
    }

    /// The negotiated `(n_sigfigs, mantissa)` must never produce a
    /// server-side tick coarser than the requested `tick_size` — that would
    /// silently widen the user's chosen granularity. The check reconstructs
    /// the implied server tick from the chosen `n`/`m`.
    #[test]
    fn compute_sigfigs_never_picks_a_coarser_tick_than_requested(
        mid in 1e-3f64..1e7f64,
        tick_ratio in 1e-6f64..1e-1f64,
    ) {
        let tick_size = mid * tick_ratio;
        let (n_opt, m_opt) = compute_sigfigs(tick_size, mid);
        if let Some(n) = n_opt {
            let m = m_opt.unwrap_or(1) as f64;
            let e = mid.log10().floor() as i32;
            let server_tick = m * 10f64.powi(e - (n as i32) + 1);
            prop_assert!(
                server_tick <= tick_size * 1.0001,
                "server tick {server_tick} exceeds requested {tick_size}"
            );
        }
    }
}
