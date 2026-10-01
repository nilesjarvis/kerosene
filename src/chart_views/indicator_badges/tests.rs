use super::active::active_chart_indicators;
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

use iced::Theme;

#[test]
fn active_indicator_registry_preserves_badge_order_and_keys() {
    let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    assert!(active_chart_indicators(&instance, &Theme::Dark).is_empty());

    instance.macro_indicators.tf_sma_50 = true;
    instance.macro_indicators.sma_50h = true;
    instance.macro_indicators.sma_200d = true;
    instance.macro_indicators.show_funding_rate = true;
    instance.macro_indicators.show_session_indicator = true;
    instance.macro_indicators.show_quick_trade = true;
    instance.macro_indicators.show_volume_profile = true;
    instance.macro_indicators.show_high_low = true;

    let active = active_chart_indicators(&instance, &Theme::Dark);
    let labels_and_keys: Vec<_> = active
        .iter()
        .map(|indicator| (indicator.label.as_str(), indicator.key.key()))
        .collect();

    assert_eq!(
        labels_and_keys,
        vec![
            ("TF 50 SMA", "tf_sma_50"),
            ("50h SMA", "sma_50h"),
            ("200d SMA", "sma_200d"),
            ("Funding", "funding_rate"),
            ("Sessions", "sessions"),
            ("Quick Trade", "quick_trade"),
            ("Vol Profile", "volume_profile"),
            ("High/Low", "high_low"),
        ]
    );
}

#[test]
fn moving_average_badges_show_custom_periods_with_stable_remove_keys() {
    let mut instance = ChartInstance::new(7, "BTC".into(), Timeframe::H1);
    instance.macro_indicators.tf_ema_50 = true;
    instance
        .macro_indicators
        .moving_average_periods
        .insert("tf_ema_50".into(), 21);
    let active = active_chart_indicators(&instance, &Theme::Dark);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].label, "TF 21 EMA");
    assert_eq!(active[0].key.key(), "tf_ema_50");
}
