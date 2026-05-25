use super::*;

#[test]
fn twap_start_schedule_validation_preserves_user_facing_messages() {
    assert_eq!(
        schedule_error_or_panic("bad", "2"),
        "Invalid TWAP duration: use 1 minute to 24 hours"
    );
    assert_eq!(
        schedule_error_or_panic("5", "bad"),
        format!(
            "Invalid TWAP slices: use 1 to {}",
            crate::twap_state::TWAP_MAX_SLICES
        )
    );
    assert_eq!(
        schedule_error_or_panic("1", "20"),
        "TWAP interval is too short: use at least 5 seconds per slice"
    );

    let schedule = parse_schedule_or_panic("5", "2");
    assert_eq!(schedule.slice_count, 2);
}

#[test]
fn twap_start_schedule_capacity_reports_combined_rate() {
    let schedule = parse_schedule_or_panic("1", "1");
    let message = schedule_capacity_error_or_panic(5.0, schedule);

    assert!(message.contains("Cannot start TWAP: active TWAP schedule is too dense"));
    assert!(message.contains("5.02 slices/sec"));
}
