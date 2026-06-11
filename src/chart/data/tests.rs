use super::SPREAD_HISTORY_LIMIT;
use crate::chart::{CandlestickChart, DEFAULT_SESSION_PANEL_HEIGHT, TIME_AXIS_HEIGHT};
use crate::timeframe::Timeframe;

#[test]
fn spread_history_is_capped_at_sample_limit() {
    let mut chart = CandlestickChart::new(1);

    for index in 0..SPREAD_HISTORY_LIMIT + 100 {
        chart.set_current_spread_at(Some(1.0 + index as f64), 1_000);
    }

    assert_eq!(chart.spread_history.len(), SPREAD_HISTORY_LIMIT);
    assert_eq!(
        chart.spread_history.front().map(|(_, spread)| *spread),
        Some(101.0)
    );
    assert_eq!(
        chart.current_spread,
        Some(SPREAD_HISTORY_LIMIT as f64 + 100.0)
    );
}

#[test]
fn missing_spread_keeps_recent_history_baseline() {
    let mut chart = CandlestickChart::new(1);
    chart.set_current_spread_at(Some(0.25), 1_000);

    chart.set_current_spread_at(None, 2_000);

    assert_eq!(chart.current_spread, None);
    assert_eq!(chart.spread_history.len(), 1);
    assert_eq!(chart.spread_history_bounds(), Some((0.25, 0.25)));
}

#[test]
fn invalid_spread_keeps_recent_history_baseline() {
    let mut chart = CandlestickChart::new(1);
    chart.set_current_spread_at(Some(0.25), 1_000);

    chart.set_current_spread_at(Some(f64::NAN), 2_000);
    chart.set_current_spread_at(Some(-0.5), 3_000);

    assert_eq!(chart.current_spread, None);
    assert_eq!(chart.spread_history.len(), 1);
}

#[test]
fn clear_spread_history_drops_current_spread_and_samples() {
    let mut chart = CandlestickChart::new(1);
    chart.set_current_spread_at(Some(0.25), 1_000);

    chart.clear_spread_history();

    assert_eq!(chart.current_spread, None);
    assert!(chart.spread_history.is_empty());
    assert_eq!(chart.spread_history_bounds(), None);
}

#[test]
fn chart_area_heights_stack_funding_and_session_panels() {
    let mut chart = CandlestickChart::new(1);
    chart.macro_indicators.show_funding_rate = true;
    chart.macro_indicators.show_session_indicator = true;

    let (chart_h, funding_h, session_h) = chart.chart_area_heights(320.0);

    assert_eq!(funding_h, chart.funding_panel_height);
    assert_eq!(session_h, DEFAULT_SESSION_PANEL_HEIGHT);
    assert_eq!(chart_h, 320.0 - TIME_AXIS_HEIGHT - funding_h - session_h);
}

#[test]
fn chart_area_heights_hide_session_panel_on_daily_timeframes() {
    let mut chart = CandlestickChart::new(1);
    chart.macro_indicators.show_session_indicator = true;
    chart.set_timeframe(Timeframe::D1);

    let (_, _, session_h) = chart.chart_area_heights(320.0);

    assert_eq!(session_h, 0.0);
}
