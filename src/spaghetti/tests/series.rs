use super::*;

#[test]
fn realtime_series_candle_update_rejects_malformed_candles() {
    let mut canvas = SpaghettiCanvas::new();
    canvas.series.push(Series {
        symbol: "BTC".to_string(),
        display: "BTC".to_string(),
        candles: vec![candle_at(1_000, 10.0)],
        color: Color::WHITE,
        loaded: true,
    });
    let mut invalid = candle_at(2_000, 20.0);
    invalid.high = 19.0;

    canvas.push_candle("BTC", invalid);

    assert_eq!(canvas.series[0].candles.len(), 1);
    assert_eq!(canvas.series[0].candles[0].close, 10.0);
}
