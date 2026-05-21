use super::{
    CustomThemeConfig, default_custom_themes, default_theme, is_known_default_hyperliquid_theme,
};

#[test]
fn default_theme_is_kerosene() {
    assert_eq!(default_theme(), "Custom: Kerosene");
}

#[test]
fn default_custom_themes_include_kerosene_chart_colors() {
    let themes = default_custom_themes();
    let kerosene = themes
        .iter()
        .find(|theme| theme.name == "Kerosene")
        .expect("Kerosene theme should be present");

    assert_eq!(kerosene.background, "#080604");
    assert_eq!(kerosene.primary, "#FF7A1A");
    assert_eq!(kerosene.chart_bull.as_deref(), Some("#FF7A1A"));
    assert_eq!(kerosene.chart_bear.as_deref(), Some("#F8EFE2"));
    assert_eq!(kerosene.success, "#FF7A1A");
    assert_eq!(kerosene.danger, "#F8EFE2");

    let inverse_eink = themes
        .iter()
        .find(|theme| theme.name == "Inverse E-Ink")
        .expect("Inverse E-Ink theme should be present");

    assert_eq!(inverse_eink.background, "#000000");
    assert_eq!(inverse_eink.text, "#ffffff");
    assert_eq!(inverse_eink.primary, "#aaaaaa");
    assert_eq!(inverse_eink.success, "#666666");
    assert_eq!(inverse_eink.warning, "#888888");
    assert_eq!(inverse_eink.danger, "#dddddd");
    assert_eq!(inverse_eink.chart_bull, None);
    assert_eq!(inverse_eink.chart_bear, None);

    let hyperliquid = themes
        .iter()
        .find(|theme| theme.name == "Hyperliquid")
        .expect("Hyperliquid theme should be present");

    assert_eq!(hyperliquid.background, "#0F1A1E");
    assert_eq!(hyperliquid.text, "#F6FEFD");
    assert_eq!(hyperliquid.primary, "#50D2C1");
    assert_eq!(hyperliquid.success, "#50D2C1");
    assert_eq!(hyperliquid.warning, "#FFB648");
    assert_eq!(hyperliquid.danger, "#ED7088");
    assert_eq!(hyperliquid.chart_bull.as_deref(), Some("#26A69A"));
    assert_eq!(hyperliquid.chart_bear.as_deref(), Some("#EF5350"));

    let xyz = themes
        .iter()
        .find(|theme| theme.name == "XYZ")
        .expect("XYZ theme should be present");

    assert_eq!(xyz.background, "#11151B");
    assert_eq!(xyz.text, "#E8E8E8");
    assert_eq!(xyz.primary, "#FFC028");
    assert_eq!(xyz.success, "#08A088");
    assert_eq!(xyz.warning, "#D8A828");
    assert_eq!(xyz.danger, "#FF3848");
    assert_eq!(xyz.chart_bull.as_deref(), Some("#08A088"));
    assert_eq!(xyz.chart_bear.as_deref(), Some("#FF3848"));

    let kraken = themes
        .iter()
        .find(|theme| theme.name == "Kraken")
        .expect("Kraken theme should be present");

    assert_eq!(kraken.background, "#0B0711");
    assert_eq!(kraken.text, "#E8E1F2");
    assert_eq!(kraken.primary, "#7132F5");
    assert_eq!(kraken.success, "#2BB67B");
    assert_eq!(kraken.warning, "#ED9B35");
    assert_eq!(kraken.danger, "#B2425F");
    assert_eq!(kraken.chart_bull.as_deref(), Some("#2BB67B"));
    assert_eq!(kraken.chart_bear.as_deref(), Some("#E34A6F"));

    let bloomberg = themes
        .iter()
        .find(|theme| theme.name == "Bloomberg")
        .expect("Bloomberg theme should be present");

    assert_eq!(bloomberg.background, "#000000");
    assert_eq!(bloomberg.text, "#F2F2E8");
    assert_eq!(bloomberg.primary, "#FF9F1A");
    assert_eq!(bloomberg.success, "#00B050");
    assert_eq!(bloomberg.warning, "#FFD84A");
    assert_eq!(bloomberg.danger, "#B00024");
    assert_eq!(bloomberg.chart_bull.as_deref(), Some("#00C853"));
    assert_eq!(bloomberg.chart_bear.as_deref(), Some("#D50032"));

    let ftx = themes
        .iter()
        .find(|theme| theme.name == "FTX")
        .expect("FTX theme should be present");

    assert_eq!(ftx.background, "#101824");
    assert_eq!(ftx.text, "#D8E2EE");
    assert_eq!(ftx.primary, "#00A8B8");
    assert_eq!(ftx.success, "#08A67A");
    assert_eq!(ftx.warning, "#F0A040");
    assert_eq!(ftx.danger, "#F03060");
    assert_eq!(ftx.chart_bull.as_deref(), Some("#08A67A"));
    assert_eq!(ftx.chart_bear.as_deref(), Some("#F03060"));

    let ibkr_dark = themes
        .iter()
        .find(|theme| theme.name == "IBKR Dark")
        .expect("IBKR Dark theme should be present");

    assert_eq!(ibkr_dark.background, "#101018");
    assert_eq!(ibkr_dark.text, "#D8DCE6");
    assert_eq!(ibkr_dark.primary, "#2878F0");
    assert_eq!(ibkr_dark.success, "#2EBF7A");
    assert_eq!(ibkr_dark.warning, "#D0A818");
    assert_eq!(ibkr_dark.danger, "#F83048");
    assert_eq!(ibkr_dark.chart_bull.as_deref(), Some("#2EBF7A"));
    assert_eq!(ibkr_dark.chart_bear.as_deref(), Some("#F83048"));

    let bybit = themes
        .iter()
        .find(|theme| theme.name == "bybit")
        .expect("bybit theme should be present");

    assert_eq!(bybit.background, "#101014");
    assert_eq!(bybit.text, "#F5F5F5");
    assert_eq!(bybit.primary, "#F4B444");
    assert_eq!(bybit.success, "#55AF72");
    assert_eq!(bybit.warning, "#E8A838");
    assert_eq!(bybit.danger, "#DC5351");
    assert_eq!(bybit.chart_bull.as_deref(), Some("#55AF72"));
    assert_eq!(bybit.chart_bear.as_deref(), Some("#DC5351"));
}

#[test]
fn known_hyperliquid_defaults_are_refreshable() {
    let legacy = CustomThemeConfig {
        name: "Hyperliquid".to_string(),
        background: "#001411".to_string(),
        text: "#F6FEFD".to_string(),
        primary: "#97FCE4".to_string(),
        success: "#97FCE4".to_string(),
        warning: "#E8D46A".to_string(),
        danger: "#FF6B6B".to_string(),
        chart_bull: Some("#97FCE4".to_string()),
        chart_bear: Some("#FF6B6B".to_string()),
    };

    assert!(is_known_default_hyperliquid_theme(&legacy));

    let legacy_without_chart_colors = CustomThemeConfig {
        chart_bull: None,
        chart_bear: None,
        ..legacy
    };

    assert!(is_known_default_hyperliquid_theme(
        &legacy_without_chart_colors
    ));

    let sampled_default = CustomThemeConfig {
        name: "Hyperliquid".to_string(),
        background: "#0F1A1F".to_string(),
        text: "#F6FEFD".to_string(),
        primary: "#50D2C1".to_string(),
        success: "#1FA67D".to_string(),
        warning: "#FFB648".to_string(),
        danger: "#ED7088".to_string(),
        chart_bull: Some("#26A69A".to_string()),
        chart_bear: Some("#EF5350".to_string()),
    };

    assert!(is_known_default_hyperliquid_theme(&sampled_default));
}
