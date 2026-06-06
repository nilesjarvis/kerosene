use super::*;

fn candle(volume: f64, close: f64) -> Candle {
    Candle::test_ohlcv(0, 0, [0.0, 0.0, 0.0, close], volume)
}

#[test]
fn outcome_volume_from_candles_sums_positive_finite_contract_and_notional_volume() {
    let candles = vec![
        candle(10.0, 0.25),
        candle(f64::NAN, 0.25),
        candle(-4.0, 0.25),
        candle(5.5, 0.50),
        candle(f64::INFINITY, 0.25),
        candle(1.0, f64::NAN),
    ];

    assert_eq!(
        outcome_volume_from_candles(&candles),
        OutcomeVolume24h {
            contract: 16.5,
            notional: 5.25,
        }
    );
}
