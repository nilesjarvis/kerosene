use super::options::all_indicator_options;
use crate::config::MacroIndicatorsConfig;

#[test]
fn indicator_menu_options_preserve_keys_and_checked_states() {
    let indicators = MacroIndicatorsConfig {
        tf_sma_50: true,
        sma_50h: true,
        sma_200d: true,
        show_funding_rate: true,
        show_session_indicator: true,
        show_labels: false,
        show_volume_profile: true,
        show_leledc_levels: true,
        ..MacroIndicatorsConfig::default()
    };

    let options = all_indicator_options(&indicators);
    let keys: Vec<_> = options.iter().map(|option| option.key).collect();
    assert_eq!(
        keys,
        vec![
            "tf_sma_50",
            "tf_ema_50",
            "tf_sma_200",
            "tf_ema_200",
            "sma_50h",
            "ema_50h",
            "sma_200h",
            "ema_200h",
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
            "show_session_indicator",
            "show_labels",
            "show_volume_profile",
            "show_leledc_arrows",
            "show_leledc_levels",
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
            "sma_50h",
            "sma_200d",
            "show_funding_rate",
            "show_session_indicator",
            "show_volume_profile",
            "show_leledc_levels",
        ]
    );
}
