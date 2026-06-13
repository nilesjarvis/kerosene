use super::planning::{funding_attempt_allowed, funding_incremental_due, funding_time_range};
use super::*;
use crate::api::Candle;
use crate::chart_state::{ChartInstance, FundingFetchMode, FundingFetchRequest};
use crate::hydromancer_api::FundingRatePoint;
use crate::timeframe::Timeframe;

fn candle(open_time: u64) -> Candle {
    Candle::test_ohlcv(open_time, open_time, [1.0, 1.0, 1.0, 1.0], 1.0)
}

#[test]
fn funding_range_uses_first_candle_and_caps_end_at_now() {
    let candles = [candle(1_000), candle(2_000)];

    assert_eq!(
        funding_time_range(&candles, 3_600_000, 3_000),
        Some((1_000, 3_000))
    );
}

#[test]
fn funding_range_waits_without_candles_or_duration() {
    assert_eq!(funding_time_range(&[], 3_600_000, 3_000), None);
    assert_eq!(funding_time_range(&[candle(1_000)], 0, 1_000), None);
}

#[test]
fn incremental_funding_waits_until_next_hourly_bucket() {
    assert!(!funding_incremental_due(1_000, 3_600_999));
    assert!(funding_incremental_due(1_000, 3_601_000));
}

#[test]
fn funding_attempts_are_throttled() {
    assert!(funding_attempt_allowed(None, 10_000, 5_000));
    assert!(!funding_attempt_allowed(Some(8_000), 10_000, 5_000));
    assert!(funding_attempt_allowed(Some(5_000), 10_000, 5_000));
}

#[test]
fn stale_hydromancer_generation_does_not_apply_funding_history() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.hydromancer_key_generation = 2;

    let request = FundingFetchRequest {
        chart_id: 7,
        symbol: "BTC".to_string(),
        coin: "BTC".to_string(),
        hydromancer_key_generation: 1,
        start_ms: 0,
        end_ms: 3_600_000,
        mode: FundingFetchMode::Snapshot,
    };
    let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    instance.macro_indicators.show_funding_rate = true;
    instance.funding_fetch_request = Some(request.clone());
    terminal.charts.insert(7, instance);

    let _task = terminal.apply_chart_funding_history_loaded(
        request.clone(),
        Ok(vec![FundingRatePoint {
            time_ms: 60_000,
            rate: 0.01,
        }]),
    );

    let instance = terminal.charts.get(&7).expect("chart instance");
    assert_eq!(instance.funding_fetch_request.as_ref(), Some(&request));
    assert!(instance.chart.funding_rates.is_empty());
}
