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

#[test]
fn outcome_volume_debug_retains_public_market_values() {
    let volume = OutcomeVolume24h {
        contract: 918_273.125,
        notional: 827_364.25,
    };

    let rendered = format!("{volume:?}");

    assert!(rendered.contains("918273.125"), "{rendered}");
    assert!(rendered.contains("827364.25"), "{rendered}");
    assert_eq!(volume.contract.to_bits(), 918_273.125_f64.to_bits());
    assert_eq!(volume.notional.to_bits(), 827_364.25_f64.to_bits());
}
