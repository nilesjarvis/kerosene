use super::{line_area_gradient, line_series_colors, visible_close_points};
use crate::api::Candle;
use crate::chart::CandlestickChart;
use crate::helpers::assert_close_loose as assert_near;
use iced::{Color, Theme};

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

#[test]
fn line_series_uses_theme_text_and_primary_without_override() {
    let chart = CandlestickChart::new(1);
    let theme = Theme::Dark;

    let (line, accent) = line_series_colors(&chart, &theme);

    assert_eq!(line, theme.palette().text);
    assert_eq!(accent, theme.extended_palette().primary.base.color);
}

#[test]
fn line_series_uses_chart_line_override_for_stroke_and_area_fallback() {
    let mut chart = CandlestickChart::new(1);
    chart.chart_line_color = Some(Color::from_rgb8(0x9A, 0xD7, 0xFF));

    let (line, accent) = line_series_colors(&chart, &Theme::Dark);

    assert_eq!(line, Color::from_rgb8(0x9A, 0xD7, 0xFF));
    assert_eq!(accent, Color::from_rgb8(0x9A, 0xD7, 0xFF));
}

#[test]
fn line_series_uses_chart_gradient_override_for_area() {
    let mut chart = CandlestickChart::new(1);
    chart.chart_line_color = Some(Color::from_rgb8(0x9A, 0xD7, 0xFF));
    chart.chart_line_gradient_color = Some(Color::from_rgb8(0x00, 0x54, 0xA6));

    let (line, accent) = line_series_colors(&chart, &Theme::Dark);

    assert_eq!(line, Color::from_rgb8(0x9A, 0xD7, 0xFF));
    assert_eq!(accent, Color::from_rgb8(0x00, 0x54, 0xA6));
}

#[test]
fn line_area_gradient_uses_stable_price_region_bounds() {
    let gradient = line_area_gradient(Color::WHITE, 420.0, 64.0);
    let stops = gradient.stops.iter().flatten().collect::<Vec<_>>();

    assert_near(gradient.start.y, 0.0);
    assert_near(gradient.end.y, 420.0);
    assert_eq!(stops.len(), 2);
    assert_near(stops[0].offset, 0.0);
    assert_near(stops[0].color.a, 0.24);
    assert_near(stops[1].offset, 1.0);
    assert_near(stops[1].color.a, 0.0);
}

#[test]
fn line_area_gradient_extends_to_projected_baseline_when_distorted() {
    let gradient = line_area_gradient(Color::WHITE, 420.0, 448.0);

    assert_near(gradient.start.y, 0.0);
    assert_near(gradient.end.y, 448.0);
}
