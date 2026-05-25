use super::candle_at;
use crate::chart::{CandlestickChart, ChartStatus};
use crate::hydromancer_api::FundingRatePoint;

#[test]
fn merge_candles_replaces_overlaps_and_keeps_sorted_order() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0), candle_at(2_000, 20.0)]);

    chart.merge_candles(vec![candle_at(3_000, 30.0), candle_at(2_000, 22.0)]);

    assert_eq!(
        chart
            .candles
            .iter()
            .map(|candle| (candle.open_time, candle.close))
            .collect::<Vec<_>>(),
        vec![(1_000, 10.0), (2_000, 22.0), (3_000, 30.0)]
    );
    assert!(matches!(chart.status, ChartStatus::Loaded));
}

#[test]
fn realtime_candle_update_rejects_malformed_candles() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 10.0)]);
    let mut invalid = candle_at(1_000, 20.0);
    invalid.close = f64::NAN;

    chart.push_candle(invalid);

    assert_eq!(chart.candles.len(), 1);
    assert_eq!(chart.candles[0].close, 10.0);
}

#[test]
fn merge_funding_history_updates_overlaps_and_keeps_sorted_order() {
    let mut chart = CandlestickChart::new(1);
    chart.set_funding_history(vec![
        FundingRatePoint {
            time_ms: 1_000,
            rate: 0.01,
        },
        FundingRatePoint {
            time_ms: 2_000,
            rate: 0.02,
        },
    ]);

    chart.merge_funding_history(vec![
        FundingRatePoint {
            time_ms: 2_000,
            rate: -0.03,
        },
        FundingRatePoint {
            time_ms: 3_000,
            rate: 0.04,
        },
    ]);

    assert_eq!(
        chart
            .funding_rates
            .iter()
            .map(|point| (point.time_ms, point.rate))
            .collect::<Vec<_>>(),
        vec![(1_000, 0.01), (2_000, -0.03), (3_000, 0.04)]
    );
}

#[test]
fn empty_incremental_funding_update_preserves_existing_points() {
    let mut chart = CandlestickChart::new(1);
    chart.set_funding_history(vec![FundingRatePoint {
        time_ms: 1_000,
        rate: 0.01,
    }]);

    chart.merge_funding_history(Vec::new());

    assert_eq!(chart.funding_rates.len(), 1);
    assert_eq!(chart.funding_rates[0].rate, 0.01);
}
