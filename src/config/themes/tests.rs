use super::{
    CustomThemeConfig, default_custom_themes, default_theme, is_known_default_bloomberg_theme,
    is_known_default_hyperliquid_theme,
};

mod bloomberg_refresh;
mod defaults;
mod hyperliquid_refresh;

struct ThemeExpectation<'a> {
    name: &'a str,
    background: &'a str,
    text: &'a str,
    primary: &'a str,
    success: &'a str,
    warning: &'a str,
    danger: &'a str,
    chart_bull: Option<&'a str>,
    chart_bear: Option<&'a str>,
    chart_line: Option<&'a str>,
}

fn theme_named<'a>(themes: &'a [CustomThemeConfig], name: &str) -> &'a CustomThemeConfig {
    match themes.iter().find(|theme| theme.name == name) {
        Some(theme) => theme,
        None => panic!("{name} theme should be present"),
    }
}

fn assert_theme_matches(themes: &[CustomThemeConfig], expected: ThemeExpectation<'_>) {
    let theme = theme_named(themes, expected.name);

    assert_eq!(theme.background, expected.background);
    assert_eq!(theme.text, expected.text);
    assert_eq!(theme.primary, expected.primary);
    assert_eq!(theme.success, expected.success);
    assert_eq!(theme.warning, expected.warning);
    assert_eq!(theme.danger, expected.danger);
    assert_eq!(theme.chart_bull.as_deref(), expected.chart_bull);
    assert_eq!(theme.chart_bear.as_deref(), expected.chart_bear);
    assert_eq!(theme.chart_line.as_deref(), expected.chart_line);
    let expected_line_gradient = if expected.name == "Bloomberg" {
        Some("#0054A6")
    } else {
        None
    };
    assert_eq!(theme.chart_line_gradient.as_deref(), expected_line_gradient);
}

fn hyperliquid_theme(
    background: &str,
    primary: &str,
    success: &str,
    chart_bull: Option<&str>,
    chart_bear: Option<&str>,
) -> CustomThemeConfig {
    CustomThemeConfig {
        name: "Hyperliquid".to_string(),
        background: background.to_string(),
        text: "#F6FEFD".to_string(),
        primary: primary.to_string(),
        success: success.to_string(),
        warning: if primary == "#97FCE4" {
            "#E8D46A".to_string()
        } else {
            "#FFB648".to_string()
        },
        danger: if primary == "#97FCE4" {
            "#FF6B6B".to_string()
        } else {
            "#ED7088".to_string()
        },
        chart_bull: chart_bull.map(str::to_string),
        chart_bear: chart_bear.map(str::to_string),
        chart_line: None,
        chart_line_gradient: None,
    }
}
