use super::planning::{
    FundingRequestPlan, funding_attempt_allowed, funding_incremental_due, funding_time_range,
    plan_funding_request,
};
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
fn funding_plan_carries_incarnation_and_wrapped_per_chart_request_id() {
    let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    instance.chart.candles = vec![candle(1_000), candle(2_000)];
    instance.funding_request_id = u64::MAX;

    let plan = plan_funding_request(
        &instance,
        false,
        Some("BTC".to_string()),
        3_000,
        9,
        instance.funding_request_id.wrapping_add(1),
        4,
        false,
    );

    let FundingRequestPlan::Fetch(request) = plan else {
        panic!("funding request should be due");
    };
    assert_eq!(request.chart_id, 7);
    assert_eq!(request.chart_instance_generation, 9);
    assert_eq!(request.request_id, 0);
    assert_eq!(request.hydromancer_key_generation, 4);
}

#[test]
fn stale_hydromancer_generation_does_not_apply_funding_history() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.hydromancer_key_generation = 2;

    let request = FundingFetchRequest {
        chart_id: 7,
        chart_instance_generation: terminal.chart_instance_generation,
        request_id: 1,
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
        }])
        .into(),
    );

    let instance = terminal.charts.get(&7).expect("chart instance");
    assert_eq!(instance.funding_fetch_request.as_ref(), Some(&request));
    assert!(instance.chart.funding_rates.is_empty());
}

#[test]
fn recreated_chart_rejects_prior_layout_funding_result() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.chart_instance_generation = 2;

    let prior_layout_request = FundingFetchRequest {
        chart_id: 7,
        chart_instance_generation: 1,
        request_id: 1,
        symbol: "BTC".to_string(),
        coin: "BTC".to_string(),
        hydromancer_key_generation: terminal.hydromancer_key_generation,
        start_ms: 0,
        end_ms: 3_600_000,
        mode: FundingFetchMode::Snapshot,
    };
    let current_request = FundingFetchRequest {
        chart_instance_generation: terminal.chart_instance_generation,
        ..prior_layout_request.clone()
    };
    let mut recreated = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    recreated.macro_indicators.show_funding_rate = true;
    recreated.funding_fetch_request = Some(current_request.clone());
    terminal.charts.insert(7, recreated);

    let _task = terminal.apply_chart_funding_history_loaded(
        prior_layout_request.clone(),
        Ok(vec![FundingRatePoint {
            time_ms: 60_000,
            rate: 0.01,
        }])
        .into(),
    );

    let recreated = terminal.charts.get(&7).expect("recreated chart");
    assert_eq!(
        recreated.funding_fetch_request.as_ref(),
        Some(&current_request)
    );
    assert!(recreated.chart.funding_rates.is_empty());
}

#[test]
fn reissued_funding_request_rejects_older_same_incarnation_result() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let prior_request = FundingFetchRequest {
        chart_id: 7,
        chart_instance_generation: terminal.chart_instance_generation,
        request_id: 1,
        symbol: "BTC".to_string(),
        coin: "BTC".to_string(),
        hydromancer_key_generation: terminal.hydromancer_key_generation,
        start_ms: 0,
        end_ms: 3_600_000,
        mode: FundingFetchMode::Snapshot,
    };
    let current_request = FundingFetchRequest {
        request_id: 2,
        ..prior_request.clone()
    };
    let mut instance = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    instance.macro_indicators.show_funding_rate = true;
    instance.funding_request_id = current_request.request_id;
    instance.funding_fetch_request = Some(current_request.clone());
    terminal.charts.insert(7, instance);

    let _task = terminal.apply_chart_funding_history_loaded(
        prior_request,
        Ok(vec![FundingRatePoint {
            time_ms: 60_000,
            rate: 0.01,
        }])
        .into(),
    );

    let instance = terminal.charts.get(&7).expect("chart instance");
    assert_eq!(
        instance.funding_fetch_request.as_ref(),
        Some(&current_request)
    );
    assert!(instance.chart.funding_rates.is_empty());
}

#[test]
fn funding_fetch_error_redacts_toast_detail() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal.hydromancer_key_generation = 1;

    let request = FundingFetchRequest {
        chart_id: 7,
        chart_instance_generation: terminal.chart_instance_generation,
        request_id: 1,
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
        request,
        Err("funding rejected: api_key=key-secret signature=sig-secret".to_string()).into(),
    );

    let instance = terminal.charts.get(&7).expect("chart instance");
    assert!(instance.funding_fetch_request.is_none());
    assert_eq!(
        instance
            .chart
            .funding_status
            .as_ref()
            .map(|(message, is_error)| (message.as_str(), *is_error)),
        Some(("Funding fetch failed", true))
    );

    let toast = terminal.toasts.last().expect("toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("api_key=<redacted>"));
    assert!(toast.message.contains("signature=<redacted>"));
    assert!(!toast.message.contains("key-secret"));
    assert!(!toast.message.contains("sig-secret"));
}
