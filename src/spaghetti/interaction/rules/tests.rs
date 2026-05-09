use super::super::super::state::{MAX_PX_PER_MS, MIN_PX_PER_MS};
use super::{
    anchored_scroll_offset_for_zoom, minimum_scroll_offset, ratio_zoom_speed,
    scroll_offset_for_zoom, zoomed_px_per_ms,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn ratio_zoom_speed_scales_with_pair_ratio_magnitude() {
    assert_close(ratio_zoom_speed(0.01), 1.03);
    assert_close(ratio_zoom_speed(0.05), 1.05);
    assert_close(ratio_zoom_speed(0.20), 1.07);
    assert_close(ratio_zoom_speed(5.0), 1.12);
    assert_close(ratio_zoom_speed(50.0), 1.16);
    assert_close(ratio_zoom_speed(500.0), 1.20);
    assert_close(ratio_zoom_speed(2_000.0), 1.24);
    assert_close(ratio_zoom_speed(2_000.01), 1.28);
}

#[test]
fn zoomed_px_per_ms_clamps_to_chart_zoom_bounds() {
    assert_eq!(zoomed_px_per_ms(MIN_PX_PER_MS, 0.1), MIN_PX_PER_MS);
    assert_eq!(zoomed_px_per_ms(MAX_PX_PER_MS, 10.0), MAX_PX_PER_MS);
}

#[test]
fn scroll_offset_for_zoom_keeps_cursor_timestamp_anchored() {
    let new_offset = scroll_offset_for_zoom(1.0, 2.0, 10.0, 100.0, 75.0);

    assert_close(new_offset, 22.5);
}

#[test]
fn anchored_scroll_offset_for_zoom_keeps_cursor_timestamp_anchored_from_left() {
    let new_offset = anchored_scroll_offset_for_zoom(1.0, 2.0, 10.0, 75.0);

    assert_close(new_offset, 47.5);
}

#[test]
fn minimum_scroll_offset_allows_unanchored_pair_ratio_overscroll_only() {
    assert_close(minimum_scroll_offset(100.0, 2.0, true, false), -37.5);
    assert_close(minimum_scroll_offset(100.0, 2.0, true, true), 0.0);
    assert_close(minimum_scroll_offset(100.0, 2.0, false, false), 0.0);
}
