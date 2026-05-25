use super::*;

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
