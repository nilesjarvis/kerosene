use serde::{Deserialize, Serialize};

mod defaults;
#[cfg(test)]
mod tests;

pub(crate) use defaults::default_custom_themes;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomThemeConfig {
    pub name: String,
    pub background: String,
    pub text: String,
    pub primary: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    #[serde(default)]
    pub chart_bull: Option<String>,
    #[serde(default)]
    pub chart_bear: Option<String>,
    #[serde(default)]
    pub chart_line: Option<String>,
}

pub fn default_theme() -> String {
    "Custom: Kerosene".to_string()
}

fn optional_hex_eq(value: &Option<String>, expected: &str) -> bool {
    value
        .as_deref()
        .is_none_or(|actual| actual.eq_ignore_ascii_case(expected))
}

pub(crate) fn is_known_default_hyperliquid_theme(theme: &CustomThemeConfig) -> bool {
    let original_default = matches!(
        theme.background.to_ascii_uppercase().as_str(),
        "#001411" | "#072723"
    ) && theme.text.eq_ignore_ascii_case("#F6FEFD")
        && theme.primary.eq_ignore_ascii_case("#97FCE4")
        && theme.success.eq_ignore_ascii_case("#97FCE4")
        && theme.warning.eq_ignore_ascii_case("#E8D46A")
        && theme.danger.eq_ignore_ascii_case("#FF6B6B")
        && optional_hex_eq(&theme.chart_bull, "#97FCE4")
        && optional_hex_eq(&theme.chart_bear, "#FF6B6B")
        && theme.chart_line.is_none();

    let sampled_default = theme.background.eq_ignore_ascii_case("#0F1A1F")
        && theme.text.eq_ignore_ascii_case("#F6FEFD")
        && theme.primary.eq_ignore_ascii_case("#50D2C1")
        && theme.success.eq_ignore_ascii_case("#1FA67D")
        && theme.warning.eq_ignore_ascii_case("#FFB648")
        && theme.danger.eq_ignore_ascii_case("#ED7088")
        && optional_hex_eq(&theme.chart_bull, "#26A69A")
        && optional_hex_eq(&theme.chart_bear, "#EF5350")
        && theme.chart_line.is_none();

    original_default || sampled_default
}

pub(crate) fn is_known_default_bloomberg_theme(theme: &CustomThemeConfig) -> bool {
    theme.background.eq_ignore_ascii_case("#000000")
        && theme.text.eq_ignore_ascii_case("#F2F2E8")
        && theme.primary.eq_ignore_ascii_case("#FF9F1A")
        && theme.success.eq_ignore_ascii_case("#00B050")
        && theme.warning.eq_ignore_ascii_case("#FFD84A")
        && theme.danger.eq_ignore_ascii_case("#B00024")
        && optional_hex_eq(&theme.chart_bull, "#00C853")
        && optional_hex_eq(&theme.chart_bear, "#D50032")
        && (optional_hex_eq(&theme.chart_line, "#9AD7FF")
            || optional_hex_eq(&theme.chart_line, "#0054A6"))
}
