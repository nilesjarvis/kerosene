use super::*;

#[test]
fn request_planner_builds_candle_derived_range() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let request = request_or_panic(context(&candles, None));

    assert_eq!(request.coin, "BTC");
    assert_eq!(request.start_time, 0);
    assert_eq!(request.end_time, 2);
    assert_near(request.min_price, 88.5);
    assert_near(request.max_price, 121.5);
}

#[test]
fn request_planner_caps_very_large_time_ranges() {
    let now = 2_000_000;
    let candles = vec![
        candle((now - 7 * 24 * 60 * 60) * 1000, 90.0, 110.0),
        candle(now * 1000, 95.0, 120.0),
    ];
    let mut ctx = context(&candles, None);
    ctx.now_time = now;

    let request = request_or_panic(ctx);

    assert_eq!(request.start_time, now - HEATMAP_MAX_REQUEST_SPAN_SECS);
    assert_eq!(request.end_time, now);
}
