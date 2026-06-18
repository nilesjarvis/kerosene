use super::*;

fn bloomberg_theme(
    chart_bull: Option<&str>,
    chart_bear: Option<&str>,
    chart_line: Option<&str>,
) -> CustomThemeConfig {
    CustomThemeConfig {
        name: "Bloomberg".to_string(),
        background: "#000000".to_string(),
        text: "#F2F2E8".to_string(),
        primary: "#FF9F1A".to_string(),
        success: "#00B050".to_string(),
        warning: "#FFD84A".to_string(),
        danger: "#B00024".to_string(),
        chart_bull: chart_bull.map(str::to_string),
        chart_bear: chart_bear.map(str::to_string),
        chart_line: chart_line.map(str::to_string),
    }
}

#[test]
fn known_bloomberg_defaults_are_refreshable() {
    let current = bloomberg_theme(Some("#00C853"), Some("#D50032"), Some("#9AD7FF"));
    assert!(is_known_default_bloomberg_theme(&current));

    let legacy_without_line = bloomberg_theme(Some("#00C853"), Some("#D50032"), None);
    assert!(is_known_default_bloomberg_theme(&legacy_without_line));

    let previous_line_default = bloomberg_theme(Some("#00C853"), Some("#D50032"), Some("#0054A6"));
    assert!(is_known_default_bloomberg_theme(&previous_line_default));

    let customized = CustomThemeConfig {
        primary: "#0080FF".to_string(),
        ..current
    };
    assert!(!is_known_default_bloomberg_theme(&customized));
}
