use super::{ChartConfig, MacroIndicatorsConfig, json_string, value_from_str};
use crate::chart_indicator::ChartIndicatorId;

#[test]
fn moving_average_periods_round_trip_and_legacy_charts_keep_defaults() {
    let legacy: ChartConfig = value_from_str(
        r#"{"id":7,"macro_indicators":{"tf_ema_50":true,"sma_200d":true}}"#,
        "legacy chart",
    );
    assert!(legacy.macro_indicators.moving_average_periods.is_empty());
    assert_eq!(
        ChartIndicatorId::TfEma50.period(&legacy.macro_indicators),
        Some(50)
    );
    let mut custom = legacy;
    custom
        .macro_indicators
        .moving_average_periods
        .insert("tf_ema_50".into(), 21);
    custom
        .macro_indicators
        .moving_average_periods
        .insert("sma_200d".into(), 89);
    let json = json_string(&custom, "custom chart");
    let restored: ChartConfig = value_from_str(&json, "restored custom chart");
    assert_eq!(restored, custom);
}

#[test]
fn moving_average_invalid_saved_periods_fall_back_to_defaults() {
    let config: MacroIndicatorsConfig = value_from_str(
        r#"{"moving_average_periods":{"tf_ema_50":0,"sma_200d":5001,"unknown":21}}"#,
        "saved periods",
    );
    assert_eq!(ChartIndicatorId::TfEma50.period(&config), Some(50));
    assert_eq!(ChartIndicatorId::Sma200d.period(&config), Some(200));
    assert_eq!(ChartIndicatorId::Labels.period(&config), None);
}
