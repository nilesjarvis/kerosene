use super::*;

fn candle(volume: f64) -> Candle {
    Candle {
        open_time: 0,
        close_time: 0,
        open: 0.0,
        high: 0.0,
        low: 0.0,
        close: 0.0,
        volume,
    }
}

#[test]
fn outcome_volume_from_candles_sums_positive_finite_contract_volume() {
    let candles = vec![
        candle(10.0),
        candle(f64::NAN),
        candle(-4.0),
        candle(5.5),
        candle(f64::INFINITY),
    ];

    assert_eq!(outcome_volume_from_candles(&candles), 15.5);
}
