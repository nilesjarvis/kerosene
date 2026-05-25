use super::*;

#[test]
fn validates_twap_duration_and_interval() {
    let duration = valid_duration_minutes("1");
    assert!(validate_twap_interval(duration, 12));
    assert!(!validate_twap_interval(duration, 13));
    assert!(parse_twap_duration_minutes("0.1").is_none());
    assert!(parse_twap_slice_count("101").is_none());
}

#[test]
fn twap_schedule_capacity_accounts_for_active_slice_rate() {
    let one_minute = Duration::from_secs(60);
    assert_eq!(twap_required_slice_rate(one_minute, 12), Some(0.2));
    assert_eq!(
        twap_required_slice_rate(Duration::ZERO, 1),
        Some(f64::INFINITY)
    );
    assert_eq!(
        twap_aggregate_slice_rate(0.8, one_minute, 12),
        Some(TWAP_MAX_AGGREGATE_SLICE_RATE)
    );
    assert!(twap_aggregate_schedule_has_capacity(0.8, one_minute, 12));
    assert!(!twap_aggregate_schedule_has_capacity(0.81, one_minute, 12));
    assert!(!twap_aggregate_schedule_has_capacity(
        f64::NAN,
        one_minute,
        12
    ));
}
