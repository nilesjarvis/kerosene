use super::*;

#[test]
fn request_planner_skips_when_previous_fetch_still_covers_range() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let previous = HeatmapFetchParams {
        coin: "BTC".to_string(),
        min_price: 88.5,
        max_price: 121.5,
        start_time: 0,
        end_time: 2,
    };

    let request = optional_request_or_panic(context(&candles, Some(&previous)));

    assert!(request.is_none());
}
