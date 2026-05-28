use super::*;

fn ubuntu_theme(background: &str) -> CustomThemeConfig {
    CustomThemeConfig {
        name: "ubuntu".to_string(),
        background: background.to_string(),
        text: "#F6F6F5".to_string(),
        primary: "#E95420".to_string(),
        success: "#2EC27E".to_string(),
        warning: "#F99B11".to_string(),
        danger: "#C7162B".to_string(),
        chart_bull: Some("#2EC27E".to_string()),
        chart_bear: Some("#C7162B".to_string()),
    }
}

#[test]
fn known_ubuntu_defaults_are_refreshable() {
    assert!(is_known_default_ubuntu_theme(&ubuntu_theme("#2C001E")));
    assert!(is_known_default_ubuntu_theme(&ubuntu_theme("#56334B")));

    let without_chart_colors = CustomThemeConfig {
        chart_bull: None,
        chart_bear: None,
        ..ubuntu_theme("#2C001E")
    };

    assert!(is_known_default_ubuntu_theme(&without_chart_colors));
}
