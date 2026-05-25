use super::active::active_chart_indicators;
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

use iced::Theme;

#[test]
fn active_indicator_registry_preserves_badge_order_and_keys() {
    let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    assert!(active_chart_indicators(&instance, &Theme::Dark).is_empty());

    instance.macro_indicators.tf_sma_50 = true;
    instance.macro_indicators.sma_200d = true;
    instance.macro_indicators.show_funding_rate = true;
    instance.macro_indicators.show_volume_profile = true;

    let active = active_chart_indicators(&instance, &Theme::Dark);
    let labels_and_keys: Vec<_> = active
        .iter()
        .map(|indicator| (indicator.label, indicator.key))
        .collect();

    assert_eq!(
        labels_and_keys,
        vec![
            ("TF 50 SMA", "tf_sma_50"),
            ("200d SMA", "sma_200d"),
            ("Funding", "show_funding_rate"),
            ("Vol Profile", "show_volume_profile"),
        ]
    );
}
