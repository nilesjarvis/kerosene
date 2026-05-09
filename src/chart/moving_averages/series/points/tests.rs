use crate::api::Candle;

use super::*;
use iced::Point;

fn candle_at(open_time: u64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    }
}

fn points_with_spacing(
    chart_candles: &[Candle],
    ma_series: &[(u64, f64)],
    first_vis: usize,
    last_vis: usize,
    chart_w: f32,
    candle_w: f32,
    spacing: f32,
) -> Vec<AveragePathPoint> {
    let idx_to_cx = |idx: usize| idx as f32 * spacing;
    let price_to_y = |value: f64| value as f32;
    visible_average_points(AveragePointContext {
        chart_candles,
        ma_series,
        first_vis,
        last_vis,
        chart_w,
        candle_w,
        idx_to_cx: &idx_to_cx,
        price_to_y: &price_to_y,
    })
}

#[test]
fn visible_average_points_use_latest_average_at_or_before_candle_time() {
    let candles = [
        candle_at(100),
        candle_at(200),
        candle_at(300),
        candle_at(400),
    ];
    let ma_series = [(50, 1.0), (250, 3.0), (400, 4.0)];

    let points = points_with_spacing(&candles, &ma_series, 0, 3, 100.0, 5.0, 10.0);

    assert_eq!(
        points,
        vec![
            AveragePathPoint {
                point: Point::new(0.0, 1.0),
                starts_segment: true,
            },
            AveragePathPoint {
                point: Point::new(10.0, 1.0),
                starts_segment: false,
            },
            AveragePathPoint {
                point: Point::new(20.0, 3.0),
                starts_segment: false,
            },
            AveragePathPoint {
                point: Point::new(30.0, 4.0),
                starts_segment: false,
            },
        ]
    );
}

#[test]
fn visible_average_points_start_new_segment_after_missing_initial_average() {
    let candles = [candle_at(100), candle_at(200), candle_at(300)];
    let ma_series = [(250, 25.0)];

    let points = points_with_spacing(&candles, &ma_series, 0, 2, 100.0, 5.0, 10.0);

    assert_eq!(
        points,
        vec![AveragePathPoint {
            point: Point::new(20.0, 25.0),
            starts_segment: true,
        }]
    );
}

#[test]
fn visible_average_points_skip_points_outside_chart_width() {
    let candles = [candle_at(100), candle_at(200), candle_at(300)];
    let ma_series = [(50, 1.0)];

    let points = points_with_spacing(&candles, &ma_series, 0, 2, 15.0, 10.0, 20.0);

    assert_eq!(
        points,
        vec![
            AveragePathPoint {
                point: Point::new(0.0, 1.0),
                starts_segment: true,
            },
            AveragePathPoint {
                point: Point::new(20.0, 1.0),
                starts_segment: false,
            },
        ]
    );
}

#[test]
fn visible_average_points_return_empty_for_empty_inputs_or_invalid_range() {
    let candles = [candle_at(100)];
    let ma_series = [(50, 1.0)];

    assert!(points_with_spacing(&[], &ma_series, 0, 0, 100.0, 5.0, 1.0).is_empty());
    assert!(points_with_spacing(&candles, &[], 0, 0, 100.0, 5.0, 1.0).is_empty());
    assert!(points_with_spacing(&candles, &ma_series, 1, 0, 100.0, 5.0, 1.0).is_empty());
}
