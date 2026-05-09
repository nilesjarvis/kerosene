use super::format_history_time_millis;

#[test]
fn history_time_uses_utc_month_day_hour_minute() {
    assert_eq!(format_history_time_millis(86_460_000), "01/02 00:01");
}

#[test]
fn history_time_handles_out_of_range_values() {
    assert_eq!(format_history_time_millis(u64::MAX), "--/-- --:--");
}
