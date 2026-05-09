use crate::app_state::TradingTerminal;
use crate::spaghetti;
use crate::timeframe::Timeframe;
use chrono::{TimeZone, Utc};

fn utc_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
    u64::try_from(
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("valid UTC timestamp")
            .timestamp_millis(),
    )
    .expect("non-negative timestamp")
}

#[test]
fn utc_week_anchor_auto_fetch_uses_intraday_granularity() {
    let now_ms = utc_ms(2026, 3, 28, 12, 0);
    let expected_start = utc_ms(2026, 3, 23, 0, 0);

    let (tf, start) = TradingTerminal::spaghetti_fetch_plan(
        Timeframe::H1,
        Some(spaghetti::Session::UtcWeek),
        None,
        now_ms,
    );

    assert_eq!(start, expected_start);
    assert_eq!(tf, Timeframe::M15);
}

#[test]
fn utc_month_anchor_auto_fetch_uses_hourly_granularity() {
    let now_ms = utc_ms(2026, 3, 28, 12, 0);
    let expected_start = utc_ms(2026, 3, 1, 0, 0);

    let (tf, start) = TradingTerminal::spaghetti_fetch_plan(
        Timeframe::H1,
        Some(spaghetti::Session::UtcMonth),
        None,
        now_ms,
    );

    assert_eq!(start, expected_start);
    assert_eq!(tf, Timeframe::H1);
}

#[test]
fn utc_year_anchor_auto_fetch_uses_granularity_that_fits_ytd() {
    let now_ms = utc_ms(2026, 8, 14, 2, 30);
    let expected_start = utc_ms(2026, 1, 1, 0, 0);

    let (tf, start) = TradingTerminal::spaghetti_fetch_plan(
        Timeframe::H1,
        Some(spaghetti::Session::UtcYear),
        None,
        now_ms,
    );

    assert_eq!(start, expected_start);
    assert_eq!(tf, Timeframe::H4);
}

#[test]
fn utc_year_anchor_rejects_manual_granularity_that_exceeds_chart_budget() {
    let now_ms = utc_ms(2026, 12, 31, 12, 0);

    let (tf, _start) = TradingTerminal::spaghetti_fetch_plan(
        Timeframe::H1,
        Some(spaghetti::Session::UtcYear),
        Some(Timeframe::M1),
        now_ms,
    );

    assert_eq!(tf, Timeframe::H4);
}
