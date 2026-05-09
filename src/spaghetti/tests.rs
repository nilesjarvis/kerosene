use super::state::DEFAULT_PX_PER_MS;
use super::{SESSION_OPTIONS, Series, Session, SpaghettiCanvas};
use crate::api::Candle;
use chrono::TimeZone;
use iced::Color;

fn ts_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
    u64::try_from(
        chrono::Utc
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("valid UTC timestamp")
            .timestamp_millis(),
    )
    .expect("non-negative timestamp")
}

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 10.0,
    }
}

#[test]
fn session_config_round_trip() {
    for &session in SESSION_OPTIONS {
        let key = session.config_str();
        assert_eq!(Session::from_config_str(key), Some(session));
    }
}

#[test]
fn realtime_series_candle_update_rejects_malformed_candles() {
    let mut canvas = SpaghettiCanvas::new();
    canvas.series.push(Series {
        symbol: "BTC".to_string(),
        display: "BTC".to_string(),
        candles: vec![candle_at(1_000, 10.0)],
        color: Color::WHITE,
        loaded: true,
    });
    let mut invalid = candle_at(2_000, 20.0);
    invalid.high = 19.0;

    canvas.push_candle("BTC", invalid);

    assert_eq!(canvas.series[0].candles.len(), 1);
    assert_eq!(canvas.series[0].candles[0].close, 10.0);
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
fn anchored_time_window_fits_full_utc_year_to_chart_width() {
    let start = ts_ms(2026, 1, 1, 0, 0);
    let end = ts_ms(2026, 12, 31, 0, 0);
    let chart_w = 720.0;

    let (left, right, _visible, px_per_ms) = super::chart_time_window(
        end,
        Some(start),
        Some(Session::UtcYear),
        0.0,
        DEFAULT_PX_PER_MS,
        chart_w,
    );

    assert_eq!(left, start as f64);
    assert_eq!(right, end as f64);
    assert!(((right - left) * px_per_ms - f64::from(chart_w)).abs() < 0.001);
}

#[test]
fn anchored_time_window_zooms_out_with_empty_space_on_right() {
    let start = ts_ms(2026, 1, 1, 0, 0);
    let end = ts_ms(2026, 1, 2, 0, 0);
    let chart_w = 720.0;

    let (left, right, _visible, px_per_ms) = super::chart_time_window(
        end,
        Some(start),
        Some(Session::UtcDay),
        0.0,
        DEFAULT_PX_PER_MS * 0.5,
        chart_w,
    );

    assert_eq!(left, start as f64);
    assert!(right > end as f64);
    assert!(((end as f64 - left) * px_per_ms - f64::from(chart_w) * 0.5).abs() < 0.001);
}

#[test]
fn anchored_time_window_allows_left_pan_but_not_right_pan() {
    let start = ts_ms(2026, 1, 1, 0, 0);
    let end = ts_ms(2026, 1, 2, 0, 0);
    let chart_w = 720.0;
    let day_ms = (end - start) as f64;

    let (left, right, _visible, _px_per_ms) = super::chart_time_window(
        end,
        Some(start),
        Some(Session::UtcDay),
        -day_ms,
        DEFAULT_PX_PER_MS * 2.0,
        chart_w,
    );

    assert_eq!(left, start as f64);
    assert_eq!(right, start as f64 + day_ms * 0.5);

    let (left, right, _visible, _px_per_ms) = super::chart_time_window(
        end,
        Some(start),
        Some(Session::UtcDay),
        day_ms,
        DEFAULT_PX_PER_MS * 2.0,
        chart_w,
    );

    assert_eq!(left, start as f64 + day_ms * 0.5);
    assert_eq!(right, end as f64);
}

#[test]
fn unanchored_time_window_keeps_user_zoom_scale() {
    let end = ts_ms(2026, 8, 14, 2, 30);
    let chart_w = 720.0;

    let (_left, _right, _visible, px_per_ms) =
        super::chart_time_window(end, None, None, 0.0, DEFAULT_PX_PER_MS, chart_w);

    assert_eq!(px_per_ms, DEFAULT_PX_PER_MS);
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
