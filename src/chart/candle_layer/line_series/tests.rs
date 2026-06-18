use super::{
    AREA_FILL_LAYERS, AREA_FILL_TOP_ALPHA, clip_polygon_to_max_y, line_area_fade_bounds,
    line_area_layer_alpha, line_series_colors, visible_close_points,
};
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
fn line_area_fade_anchors_to_series_area_when_tall_enough() {
    let (start_y, end_y) = line_area_fade_bounds(64.0, 420.0, 420.0).unwrap();

    assert_near(start_y, 64.0);
    assert_near(end_y, 420.0);
}

#[test]
fn line_area_fade_extends_short_extreme_areas_for_smooth_fade() {
    let (start_y, end_y) = line_area_fade_bounds(340.0, 420.0, 420.0).unwrap();

    assert_near(start_y, 189.0);
    assert_near(end_y, 420.0);
}

#[test]
fn line_area_fade_extends_to_projected_baseline_when_distorted() {
    let (start_y, end_y) = line_area_fade_bounds(120.0, 448.0, 420.0).unwrap();

    assert_near(start_y, 120.0);
    assert_near(end_y, 448.0);
}

#[test]
fn line_area_layer_alpha_composes_to_top_alpha() {
    let layer_alpha = line_area_layer_alpha();
    let composed = 1.0 - (1.0 - layer_alpha).powf(AREA_FILL_LAYERS as f32);

    assert_near(composed, AREA_FILL_TOP_ALPHA);
}

#[test]
fn clips_area_polygon_to_horizontal_fade_layer() {
    let polygon = vec![
        iced::Point::new(0.0, 100.0),
        iced::Point::new(0.0, 0.0),
        iced::Point::new(10.0, 0.0),
        iced::Point::new(10.0, 100.0),
    ];

    let clipped = clip_polygon_to_max_y(&polygon, 50.0);

    assert_eq!(clipped.len(), 4);
    assert_near(clipped[0].x, 0.0);
    assert_near(clipped[0].y, 50.0);
    assert_near(clipped[1].x, 0.0);
    assert_near(clipped[1].y, 0.0);
    assert_near(clipped[2].x, 10.0);
    assert_near(clipped[2].y, 0.0);
    assert_near(clipped[3].x, 10.0);
    assert_near(clipped[3].y, 50.0);
}

#[test]
fn clipping_returns_empty_when_polygon_is_below_layer() {
    let polygon = vec![
        iced::Point::new(0.0, 100.0),
        iced::Point::new(0.0, 80.0),
        iced::Point::new(10.0, 80.0),
        iced::Point::new(10.0, 100.0),
    ];

    assert!(clip_polygon_to_max_y(&polygon, 50.0).is_empty());
}
