use super::*;

#[test]
fn known_hyperliquid_defaults_are_refreshable() {
    let legacy = hyperliquid_theme(
        "#001411",
        "#97FCE4",
        "#97FCE4",
        Some("#97FCE4"),
        Some("#FF6B6B"),
    );

    assert!(is_known_default_hyperliquid_theme(&legacy));

    let legacy_without_chart_colors = CustomThemeConfig {
        chart_bull: None,
        chart_bear: None,
        ..legacy
    };

    assert!(is_known_default_hyperliquid_theme(
        &legacy_without_chart_colors
    ));

    let sampled_default = hyperliquid_theme(
        "#0F1A1F",
        "#50D2C1",
        "#1FA67D",
        Some("#26A69A"),
        Some("#EF5350"),
    );

    assert!(is_known_default_hyperliquid_theme(&sampled_default));
}
