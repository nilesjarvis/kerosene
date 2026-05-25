use super::*;

#[test]
fn request_planner_reports_out_of_range_history() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];
    let mut ctx = context(&candles, None);
    ctx.now_time = 0;

    let error = error_or_panic(ctx);

    assert_eq!(error, "HEAT only has recent HyperDash history");
}
