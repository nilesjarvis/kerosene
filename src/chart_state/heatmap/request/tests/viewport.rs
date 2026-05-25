use super::*;

#[test]
fn request_planner_uses_viewport_price_range_when_available() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let mut ctx = context(&candles, None);
    ctx.viewport = Some(ChartViewport {
        start_time_ms: 1_000,
        end_time_ms: 2_000,
        price_lo: 50.0,
        price_hi: 150.0,
        chart_width: 0.0,
        candle_width: 0.0,
        scroll_offset: 0.0,
        y_auto: true,
        y_scale: 1.0,
        y_offset: 0.0,
        funding_y_scale: 1.0,
        funding_y_offset: 0.0,
    });

    let request = request_or_panic(ctx);

    assert_near(request.min_price, 45.0);
    assert_near(request.max_price, 155.0);
}
