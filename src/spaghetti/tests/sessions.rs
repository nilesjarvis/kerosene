use super::*;

#[test]
fn session_config_round_trip() {
    for &session in SESSION_OPTIONS {
        let key = session.config_str();
        assert_eq!(Session::from_config_str(key), Some(session));
    }
}

#[test]
fn utc_day_anchor_uses_midnight() {
    let reference = ts_ms(2026, 3, 28, 15, 42);
    let expected = ts_ms(2026, 3, 28, 0, 0);
    assert_eq!(Session::UtcDay.last_open_ms(reference), expected);
}

#[test]
fn utc_week_anchor_uses_monday_boundary() {
    let saturday = ts_ms(2026, 3, 28, 11, 0);
    let monday = ts_ms(2026, 3, 23, 0, 0);
    assert_eq!(Session::UtcWeek.last_open_ms(saturday), monday);
}

#[test]
fn utc_month_anchor_uses_first_day() {
    let reference = ts_ms(2026, 3, 28, 15, 42);
    let expected = ts_ms(2026, 3, 1, 0, 0);
    assert_eq!(Session::UtcMonth.last_open_ms(reference), expected);
}

#[test]
fn utc_year_anchor_uses_january_first() {
    let reference = ts_ms(2026, 8, 14, 2, 30);
    let expected = ts_ms(2026, 1, 1, 0, 0);
    assert_eq!(Session::UtcYear.last_open_ms(reference), expected);
}

#[test]
fn us_session_anchor_handles_dst_offset() {
    // 2026-07-14 16:00 UTC == 12:00 ET (DST), so US open is 13:30 UTC.
    let reference = ts_ms(2026, 7, 14, 16, 0);
    let expected = ts_ms(2026, 7, 14, 13, 30);
    assert_eq!(Session::US.last_open_ms(reference), expected);
}

#[test]
fn us_session_anchor_handles_winter_offset() {
    // 2026-01-14 16:00 UTC == 11:00 ET, so US open is 14:30 UTC.
    let reference = ts_ms(2026, 1, 14, 16, 0);
    let expected = ts_ms(2026, 1, 14, 14, 30);
    assert_eq!(Session::US.last_open_ms(reference), expected);
}

#[test]
fn europe_session_anchor_handles_winter_offset() {
    // 2026-01-14 10:00 UTC == 10:00 London, so EU open is 08:00 UTC.
    let reference = ts_ms(2026, 1, 14, 10, 0);
    let expected = ts_ms(2026, 1, 14, 8, 0);
    assert_eq!(Session::Europe.last_open_ms(reference), expected);
}

#[test]
fn europe_session_anchor_handles_summer_offset() {
    // 2026-07-14 10:00 UTC == 11:00 London (BST), so EU open is 07:00 UTC.
    let reference = ts_ms(2026, 7, 14, 10, 0);
    let expected = ts_ms(2026, 7, 14, 7, 0);
    assert_eq!(Session::Europe.last_open_ms(reference), expected);
}
