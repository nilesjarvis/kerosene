use super::{secondary_close_points, secondary_visible_price_stats};
use crate::api::Candle;
use crate::chart::ChartState;
use crate::helpers::assert_close_loose as assert_near;

fn candle(open_time: u64, close: f64) -> Candle {
    Candle::test_ohlcv(
        open_time,
        open_time + 59_999,
        [close, close, close, close],
        1.0,
    )
}

#[test]
fn secondary_visible_price_stats_use_visible_time_range_only() {
    let candles = vec![
        candle(0, 100.0),
        candle(60_000, 110.0),
        candle(120_000, 200.0),
    ];

    let stats = secondary_visible_price_stats(&candles, 1, 90_000).expect("visible stats");

    assert_near(stats.price_lo, 99.6);
    assert_near(stats.price_hi, 110.4);
}

#[test]
fn secondary_points_map_by_timestamp_and_clip_to_chart_width() {
    let state = ChartState {
        candle_width: 10.0,
        ..Default::default()
    };
    let candles = vec![
        candle(0, 100.0),
        candle(60_000, 110.0),
        candle(120_000, 120.0),
    ];

    let points = secondary_close_points(
        &candles,
        &state,
        100.0,
        |ts| match ts {
            0 => Some(-20.0),
            60_000 => Some(50.0),
            120_000 => Some(102.0),
            _ => None,
        },
        &|price| (200.0 - price) as f32,
    );

    assert_eq!(points.len(), 2);
    assert_near(points[0].x, 50.0);
    assert_near(points[0].y, 90.0);
    assert_near(points[1].x, 102.0);
    assert_near(points[1].y, 80.0);
}
