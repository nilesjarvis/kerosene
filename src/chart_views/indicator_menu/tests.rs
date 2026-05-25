use super::options::all_indicator_options;
use crate::config::MacroIndicatorsConfig;

#[test]
fn indicator_menu_options_preserve_keys_and_checked_states() {
    let mut indicators = MacroIndicatorsConfig::default();
    indicators.tf_sma_50 = true;
    indicators.sma_200d = true;
    indicators.show_funding_rate = true;
    indicators.show_labels = false;
    indicators.show_volume_profile = true;

    let options = all_indicator_options(&indicators);
    let keys: Vec<_> = options.iter().map(|option| option.key).collect();
    assert_eq!(
        keys,
        vec![
            "tf_sma_50",
            "tf_ema_50",
            "tf_sma_200",
            "tf_ema_200",
            "sma_50d",
            "ema_50d",
            "sma_200d",
            "ema_200d",
            "sma_20w",
            "ema_20w",
            "sma_50w",
            "ema_50w",
            "sma_12m",
            "ema_12m",
            "show_funding_rate",
            "show_labels",
            "show_volume_profile",
        ]
    );

    let checked: Vec<_> = options
        .iter()
        .filter(|option| option.checked)
        .map(|option| option.key)
        .collect();
    assert_eq!(
        checked,
        vec![
            "tf_sma_50",
            "sma_200d",
            "show_funding_rate",
            "show_volume_profile",
        ]
    );
}
