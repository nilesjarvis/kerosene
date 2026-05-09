use super::{CustomThemeConfig, default_custom_themes, is_known_default_hyperliquid_theme};

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
