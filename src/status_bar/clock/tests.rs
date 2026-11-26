use super::{market_session_summary, market_session_tooltip};
use crate::market_sessions::{MARKET_CLOCK_SESSIONS, MarketSession};
use chrono::{DateTime, Utc};

fn utc_at(instant: &str) -> DateTime<Utc> {
    instant.parse().expect("valid UTC test timestamp")
}

fn summary_at(instant: &str) -> String {
    let clocks = MARKET_CLOCK_SESSIONS.map(|session| session.clock_at(utc_at(instant)));
    market_session_summary(&clocks)
}

fn tooltip_at(session: MarketSession, instant: &str) -> String {
    session.clock_at(utc_at(instant)).tooltip_line()
}

#[test]
fn regular_sessions_keep_local_hours_and_weekends() {
    assert_eq!(summary_at("2026-05-18T15:00:00Z"), "London + New York open");
    assert_eq!(summary_at("2026-05-17T15:00:00Z"), "Markets closed");
    assert_eq!(
        tooltip_at(MarketSession::NewYork, "2026-05-18T13:00:00Z"),
        "CLOSED  New York 09:00:00 EDT (opens in 0h 30m)"
    );
    assert_eq!(
        tooltip_at(MarketSession::NewYork, "2026-05-18T14:00:00Z"),
        "OPEN  New York 10:00:00 EDT (closes in 6h 0m)"
    );
    assert_eq!(
        tooltip_at(MarketSession::Asia, "2026-05-18T06:10:00Z"),
        "OPEN  Asia 15:10:00 JST (closes in 0h 20m)"
    );
    assert_eq!(summary_at("2026-05-18T06:30:00Z"), "Markets closed");
}

#[test]
fn thanksgiving_closes_only_new_york_and_counts_down_to_friday() {
    let clocks =
        MARKET_CLOCK_SESSIONS.map(|session| session.clock_at(utc_at("2026-11-26T15:00:00Z")));
    assert_eq!(market_session_summary(&clocks), "London open");
    assert!(
        market_session_tooltip(&clocks)
            .contains("CLOSED  New York 10:00:00 EST (Thanksgiving, opens in 23h 30m)")
    );
    // A US bank holiday can still be a normal exchange session.
    assert_eq!(summary_at("2026-11-11T15:00:00Z"), "London + New York open");
}

#[test]
fn thanksgiving_friday_uses_the_early_close_for_both_status_and_countdown() {
    let ny = MarketSession::NewYork;
    assert_eq!(
        tooltip_at(ny, "2026-11-27T17:59:00Z"),
        "OPEN  New York 12:59:00 EST (closes in 0h 1m)"
    );
    assert_eq!(
        tooltip_at(ny, "2026-11-27T18:00:00Z"),
        "CLOSED  New York 13:00:00 EST (opens in 2d 20h 30m)"
    );
    assert_eq!(summary_at("2026-11-27T18:00:00Z"), "Markets closed");
}

#[test]
fn london_christmas_half_day_skips_the_observed_boxing_day() {
    let london = MarketSession::London;
    assert_eq!(
        tooltip_at(london, "2026-12-24T12:29:00Z"),
        "OPEN  London 12:29:00 GMT (closes in 0h 1m)"
    );
    assert_eq!(
        tooltip_at(london, "2026-12-24T12:30:00Z"),
        "CLOSED  London 12:30:00 GMT (opens in 4d 19h 30m)"
    );
}

#[test]
fn tokyo_uses_the_local_holiday_date_and_skips_consecutive_days() {
    // Sunday UTC is already Monday in Tokyo, during Golden Week.
    assert_eq!(
        tooltip_at(MarketSession::Asia, "2026-05-03T23:30:00Z"),
        "CLOSED  Asia 08:30:00 JST (Greenery Day, opens in 3d 0h 30m)"
    );
    // Count down across the New Year and multiple exchange-only holidays.
    assert_eq!(
        tooltip_at(MarketSession::Asia, "2025-12-30T06:30:00Z"),
        "CLOSED  Asia 15:30:00 JST (opens in 5d 17h 30m)"
    );
}

#[test]
fn london_easter_countdown_uses_the_new_dst_offset() {
    assert_eq!(
        tooltip_at(MarketSession::London, "2024-03-28T16:30:00Z"),
        "UNKNOWN  London 16:30:00 GMT (calendar unavailable)"
    );
    // The UK clock changes between the Thursday close and Tuesday reopen in 2027.
    assert_eq!(
        tooltip_at(MarketSession::London, "2027-03-25T16:30:00Z"),
        "CLOSED  London 16:30:00 GMT (opens in 4d 14h 30m)"
    );
}

#[test]
fn coverage_end_never_predicts_an_unverified_open() {
    assert_eq!(
        tooltip_at(MarketSession::NewYork, "2035-12-31T21:00:00Z"),
        "CLOSED  New York 16:00:00 EST (next open unavailable)"
    );
    assert_eq!(
        summary_at("2036-01-02T15:00:00Z"),
        "Market hours unavailable"
    );
}
