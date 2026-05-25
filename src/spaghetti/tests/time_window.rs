use super::*;

#[test]
fn anchored_time_window_fits_full_utc_year_to_chart_width() {
    let start = ts_ms(2026, 1, 1, 0, 0);
    let end = ts_ms(2026, 12, 31, 0, 0);
    let chart_w = 720.0;

    let (left, right, _visible, px_per_ms) = super::super::chart_time_window(
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

    let (left, right, _visible, px_per_ms) = super::super::chart_time_window(
        end,
        Some(start),
        Some(Session::UtcDay),
        0.0,
        DEFAULT_PX_PER_MS * 0.5,
        chart_w,
    );

    let filled_width = (end as f64 - left) * px_per_ms;

    assert_eq!(left, start as f64);
    assert!(right > end as f64);
    assert!((filled_width - f64::from(chart_w) * 0.5).abs() < 0.001);
}

#[test]
fn anchored_time_window_allows_left_pan_but_not_right_pan() {
    let start = ts_ms(2026, 1, 1, 0, 0);
    let end = ts_ms(2026, 1, 2, 0, 0);
    let chart_w = 720.0;
    let day_ms = (end - start) as f64;

    let (left, right, _visible, _px_per_ms) = super::super::chart_time_window(
        end,
        Some(start),
        Some(Session::UtcDay),
        -day_ms,
        DEFAULT_PX_PER_MS * 2.0,
        chart_w,
    );

    assert_eq!(left, start as f64);
    assert_eq!(right, start as f64 + day_ms * 0.5);

    let (left, right, _visible, _px_per_ms) = super::super::chart_time_window(
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
        super::super::chart_time_window(end, None, None, 0.0, DEFAULT_PX_PER_MS, chart_w);

    assert_eq!(px_per_ms, DEFAULT_PX_PER_MS);
}
