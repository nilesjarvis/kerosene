use super::*;
use crate::market_state::{
    DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT, MAX_ORDER_BOOK_SPREAD_CHART_HEIGHT,
    MIN_ORDER_BOOK_SPREAD_CHART_HEIGHT,
};

#[test]
fn wheel_resized_height_grows_and_shrinks_from_hover_scroll() {
    assert_eq!(
        wheel_resized_height(60.0, &mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }),
        78.0
    );
    assert_eq!(
        wheel_resized_height(60.0, &mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 }),
        42.0
    );
}

#[test]
fn wheel_resized_height_normalizes_pixel_scroll() {
    assert_eq!(
        wheel_resized_height(60.0, &mouse::ScrollDelta::Pixels { x: 0.0, y: 28.0 }),
        78.0
    );
}

#[test]
fn clamped_height_respects_bounds_and_non_finite_values() {
    assert_eq!(clamped_height(10.0), MIN_ORDER_BOOK_SPREAD_CHART_HEIGHT);
    assert_eq!(clamped_height(2_000.0), MAX_ORDER_BOOK_SPREAD_CHART_HEIGHT);
    assert_eq!(
        clamped_height(f32::NAN),
        DEFAULT_ORDER_BOOK_SPREAD_CHART_HEIGHT
    );
}
