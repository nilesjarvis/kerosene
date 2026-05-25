use super::{
    format_candle_countdown, next_candle_countdown_label, point_in_axis_corner,
    remaining_ms_until_next_candle,
};
use crate::timeframe::Timeframe;
use iced::Point;

#[test]
fn countdown_uses_last_candle_open_as_anchor() {
    let last_open = 1_000_000;
    let interval = Timeframe::M1.duration_ms();

    assert_eq!(
        remaining_ms_until_next_candle(last_open, interval, last_open),
        Some(60_000)
    );
    assert_eq!(
        remaining_ms_until_next_candle(last_open, interval, last_open + 15_000),
        Some(45_000)
    );
    assert_eq!(
        remaining_ms_until_next_candle(last_open, interval, last_open + 60_000),
        Some(0)
    );
    assert_eq!(
        remaining_ms_until_next_candle(last_open, interval, last_open + 60_001),
        Some(59_999)
    );
}

#[test]
fn candle_countdown_label_is_compact() {
    assert_eq!(
        next_candle_countdown_label(1_000_000, Timeframe::M5, 1_060_000),
        Some("4m 0s".to_string())
    );
    assert_eq!(format_candle_countdown(42_000), "42s");
    assert_eq!(format_candle_countdown(90_000), "1m 30s");
    assert_eq!(format_candle_countdown(3_660_000), "1h 1m");
    assert_eq!(format_candle_countdown(90_000_000), "1d 1h");
}

#[test]
fn hover_target_is_the_axis_corner_only() {
    assert!(point_in_axis_corner(
        Point::new(420.0, 276.0),
        400.0,
        260.0,
        70.0,
        24.0,
    ));
    assert!(!point_in_axis_corner(
        Point::new(399.0, 276.0),
        400.0,
        260.0,
        70.0,
        24.0,
    ));
    assert!(!point_in_axis_corner(
        Point::new(420.0, 259.0),
        400.0,
        260.0,
        70.0,
        24.0,
    ));
}
