use super::{market_clock_text, market_is_active, status_clock_times};
use chrono::{TimeZone, Utc};

#[test]
fn market_activity_uses_local_session_hours() {
    assert!(market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::America::New_York,
        (9, 30),
        (16, 0),
    ));
    assert!(!market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::America::New_York,
        (9, 30),
        (16, 0),
    ));
    assert!(!market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 18, 20, 30, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::America::New_York,
        (9, 30),
        (16, 0),
    ));
}

#[test]
fn market_activity_respects_weekends() {
    assert!(!market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 16, 14, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::America::New_York,
        (9, 30),
        (16, 0),
    ));
}

#[test]
fn market_activity_handles_regional_timezones() {
    assert!(market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 18, 9, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::Europe::London,
        (8, 0),
        (16, 30),
    ));
    assert!(market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 18, 1, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::Asia::Tokyo,
        (9, 0),
        (15, 0),
    ));
    assert!(!market_is_active(
        Utc.with_ymd_and_hms(2026, 5, 18, 7, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::Asia::Tokyo,
        (9, 0),
        (15, 0),
    ));
}

#[test]
fn market_clock_text_shows_close_countdown_while_active() {
    let label = market_clock_text(
        "New York",
        Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::America::New_York,
        (9, 30),
        (16, 0),
    );

    assert_eq!(label, "New York 10:00:00 EDT (closes in 6h 0m)");
}

#[test]
fn market_clock_text_shows_open_countdown_while_inactive() {
    let label = market_clock_text(
        "New York",
        Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0)
            .single()
            .expect("valid UTC timestamp"),
        chrono_tz::America::New_York,
        (9, 30),
        (16, 0),
    );

    assert_eq!(label, "New York 09:00:00 EDT (opens in 0h 30m)");
}

#[test]
fn status_clock_times_uses_supplied_timestamp() {
    let expected_utc = Utc
        .with_ymd_and_hms(2026, 5, 18, 14, 0, 0)
        .single()
        .expect("valid UTC timestamp");

    let (utc, local) = status_clock_times(expected_utc.timestamp_millis() as u64);

    assert_eq!(utc, expected_utc);
    assert_eq!(local.timestamp_millis(), expected_utc.timestamp_millis());
}
