use super::{ChartConfig, MacroIndicatorsConfig, json_string, value_from_str};
use crate::config::{EmaCloudColor, EmaCloudConfig, EmaCloudTimeframe, MAX_EMA_CLOUDS};

#[test]
fn ema_clouds_round_trip_and_legacy_layouts_stay_empty() {
    let mut chart: ChartConfig = value_from_str(
        r#"{"id":7,"macro_indicators":{"tf_ema_50":true}}"#,
        "legacy chart",
    );
    assert!(chart.macro_indicators.ema_clouds.is_empty());
    assert!(!json_string(&chart, "legacy round trip").contains("ema_clouds"));
    chart.macro_indicators.ema_clouds = vec![EmaCloudConfig {
        id: 17,
        enabled: false,
        fast_period: 21,
        slow_period: 89,
        timeframe: EmaCloudTimeframe::Week,
        color: EmaCloudColor::Warning,
        opacity: 31,
    }];
    let restored: ChartConfig =
        value_from_str(&json_string(&chart, "cloud chart"), "restored chart");
    assert_eq!(restored, chart);
    let defaults: MacroIndicatorsConfig =
        value_from_str(r#"{"ema_clouds":[{"id":1}]}"#, "cloud defaults");
    assert_eq!(
        defaults.ema_clouds,
        vec![EmaCloudConfig {
            id: 1,
            ..Default::default()
        }]
    );
}

#[test]
fn ema_clouds_discard_invalid_and_duplicate_saved_entries_and_bound_count() {
    let config: MacroIndicatorsConfig = value_from_str(
        r#"{"ema_clouds":[
        {"id":1,"fast_period":0}, {"id":2,"slow_period":5001},
        {"id":3,"opacity":101}, {"id":4,"fast_period":1,"slow_period":5000,"opacity":0},
        {"id":4,"fast_period":9}, {"id":5,"opacity":100}
    ]}"#,
        "invalid clouds",
    );
    assert_eq!(
        config
            .ema_clouds
            .iter()
            .map(|cloud| cloud.id)
            .collect::<Vec<_>>(),
        vec![4, 5]
    );
    let config = MacroIndicatorsConfig {
        ema_clouds: (0..20)
            .map(|id| EmaCloudConfig {
                id,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let bounded: MacroIndicatorsConfig =
        value_from_str(&json_string(&config, "many clouds"), "bounded clouds");
    assert_eq!(bounded.ema_clouds.len(), MAX_EMA_CLOUDS);
}
