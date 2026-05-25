use super::*;

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
