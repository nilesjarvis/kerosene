use super::*;

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
