use super::visible_close_points;
use crate::api::Candle;

fn candles_with_closes(closes: &[f64]) -> Vec<Candle> {
    closes
        .iter()
        .enumerate()
        .map(|(i, close)| Candle::test_price(i as u64 * 60_000, *close))
        .collect()
}

#[test]
fn collects_close_price_points_for_visible_candles() {
    let candles = candles_with_closes(&[10.0, 11.0, 12.0]);
    let idx_to_cx = |i: usize| i as f32 * 10.0;
    let price_to_y = |value: f64| value as f32;

    let points = visible_close_points(&candles, 0, 2, 8.0, 1_000.0, &idx_to_cx, &price_to_y);

    assert_eq!(points.len(), 3);
    assert_eq!(points[0].x, 0.0);
    assert_eq!(points[0].y, 10.0);
    assert_eq!(points[2].x, 20.0);
    assert_eq!(points[2].y, 12.0);
}

#[test]
fn skips_candles_outside_the_horizontal_plot() {
    let candles = candles_with_closes(&[10.0, 11.0, 12.0, 13.0]);
    // Each candle is 100px apart; only indices whose body overlaps [0, 150]
    // should survive (a half-width of 4px keeps index 0 and 1 visible).
    let idx_to_cx = |i: usize| i as f32 * 100.0;
    let price_to_y = |value: f64| value as f32;

    let points = visible_close_points(&candles, 0, 3, 8.0, 150.0, &idx_to_cx, &price_to_y);

    assert_eq!(points.len(), 2);
    assert_eq!(points[0].x, 0.0);
    assert_eq!(points[1].x, 100.0);
}

#[test]
fn empty_for_no_candles_or_inverted_range() {
    let idx_to_cx = |i: usize| i as f32;
    let price_to_y = |value: f64| value as f32;

    assert!(visible_close_points(&[], 0, 0, 8.0, 100.0, &idx_to_cx, &price_to_y).is_empty());

    let candles = candles_with_closes(&[10.0, 11.0]);
    assert!(visible_close_points(&candles, 2, 1, 8.0, 100.0, &idx_to_cx, &price_to_y).is_empty());

    // first_vis past the end (after clamping last_vis) must not panic.
    assert!(visible_close_points(&candles, 5, 9, 8.0, 100.0, &idx_to_cx, &price_to_y).is_empty());
}

#[test]
fn clamps_last_visible_index_to_candle_bounds() {
    let candles = candles_with_closes(&[10.0, 11.0]);
    let idx_to_cx = |i: usize| i as f32 * 10.0;
    let price_to_y = |value: f64| value as f32;

    // last_vis past the end must not panic and must clamp to the final candle.
    let points = visible_close_points(&candles, 0, 9, 8.0, 1_000.0, &idx_to_cx, &price_to_y);

    assert_eq!(points.len(), 2);
    assert_eq!(points[1].y, 11.0);
}
