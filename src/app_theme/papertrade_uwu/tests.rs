use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use iced::Color;

#[test]
fn papertrade_uwu_uses_reference_surfaces_and_chart_colors() {
    let (terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig {
        active_theme: "Custom: Papertrade UwU".to_string(),
        ..KeroseneConfig::default()
    });
    let theme = terminal.theme();
    let palette = theme.palette();
    let extended = theme.extended_palette();

    assert!(TradingTerminal::palette_matches_papertrade_uwu_source(
        palette
    ));
    assert_eq!(extended.background.base.color, palette.background);
    assert_eq!(
        extended.background.strong.color,
        Color::from_rgb8(0x17, 0x1D, 0x3D)
    );
    assert_eq!(
        extended.background.weak.color,
        Color::from_rgb8(0x11, 0x18, 0x32)
    );
    assert_eq!(
        extended.background.weak.text,
        Color::from_rgb8(0xC7, 0xC4, 0xD8)
    );
    assert_eq!(extended.primary.base.color, palette.primary);
    assert_eq!(
        extended.primary.strong.color,
        Color::from_rgb8(0xC5, 0xC9, 0xF2)
    );
    assert_eq!(
        extended.background.stronger.color,
        Color::from_rgb8(0x27, 0x2F, 0x5A)
    );
    assert_eq!(
        extended.primary.base.text,
        Color::from_rgb8(0x0E, 0x13, 0x29)
    );
    assert_eq!(extended.success.base.color, palette.success);
    assert_eq!(extended.warning.base.color, palette.warning);
    assert_eq!(extended.danger.base.color, palette.danger);
    assert!(extended.is_dark);

    let overrides = terminal.active_chart_theme_overrides();
    assert_eq!(overrides.bull, Some(Color::from_rgb8(0x82, 0xDE, 0xB0)));
    assert_eq!(overrides.bear, Some(Color::from_rgb8(0xFF, 0x91, 0xA7)));
    assert_eq!(overrides.line, Some(Color::from_rgb8(0xF1, 0xF3, 0xFF)));
    assert_eq!(
        overrides.line_gradient,
        Some(Color::from_rgb8(0xDC, 0xE5, 0xFF))
    );
    assert_eq!(
        terminal.direction_colors(&theme),
        (
            Color::from_rgb8(0x82, 0xDE, 0xB0),
            Color::from_rgb8(0xFF, 0x91, 0xA7)
        )
    );
}

#[test]
fn papertrade_uwu_customizations_use_generated_surfaces() {
    for field in [
        "background",
        "text",
        "primary",
        "success",
        "warning",
        "danger",
    ] {
        let mut config = KeroseneConfig::default();
        let papertrade_uwu = config
            .custom_themes
            .iter_mut()
            .find(|theme| theme.name == "Papertrade UwU")
            .expect("Papertrade UwU preset");
        let mut value = serde_json::to_value(&*papertrade_uwu).expect("theme serializes");
        value[field] = serde_json::json!("#123456");
        *papertrade_uwu = serde_json::from_value(value).expect("customized theme deserializes");

        let (terminal, _) = TradingTerminal::boot_from_config(config);
        let theme = terminal.get_theme_by_name("Custom: Papertrade UwU");

        assert!(
            !TradingTerminal::palette_matches_papertrade_uwu_source(theme.palette()),
            "customized {field} must disable the source palette"
        );
        assert_ne!(
            theme.extended_palette().background.weak,
            TradingTerminal::papertrade_uwu_source_extended_palette()
                .background
                .weak,
            "customized {field} must use generated surfaces"
        );
    }
}
