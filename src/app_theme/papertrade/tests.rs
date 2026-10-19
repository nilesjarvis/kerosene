use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use iced::Color;

#[test]
fn papertrade_uses_reference_surfaces_and_chart_colors() {
    let (terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig {
        active_theme: "Custom: Papertrade".to_string(),
        ..KeroseneConfig::default()
    });
    let theme = terminal.theme();
    let palette = theme.palette();
    let extended = theme.extended_palette();

    assert!(TradingTerminal::palette_matches_papertrade_source(palette));
    assert_eq!(extended.background.base.color, palette.background);
    assert_eq!(extended.background.strong.color, palette.background);
    assert_eq!(
        extended.background.weak.color,
        Color::from_rgb8(0xED, 0xE5, 0xD0)
    );
    assert_eq!(
        extended.background.weak.text,
        Color::from_rgb8(0x7A, 0x70, 0x60)
    );
    assert_eq!(extended.primary.base.color, palette.primary);
    assert_eq!(extended.primary.base.text, palette.background);
    assert_eq!(extended.success.base.color, palette.success);
    assert_eq!(extended.warning.base.color, palette.warning);
    assert_eq!(extended.danger.base.color, palette.danger);
    assert!(!extended.is_dark);

    let overrides = terminal.active_chart_theme_overrides();
    assert_eq!(overrides.bull, Some(Color::from_rgb8(0x2D, 0x5F, 0x3A)));
    assert_eq!(overrides.bear, Some(Color::from_rgb8(0x8B, 0x20, 0x20)));
    assert_eq!(overrides.line, Some(Color::from_rgb8(0x2A, 0x25, 0x20)));
    assert_eq!(
        overrides.line_gradient,
        Some(Color::from_rgb8(0xD4, 0xCB, 0xBA))
    );
    assert_eq!(
        terminal.direction_colors(&theme),
        (palette.success, palette.danger)
    );
}

#[test]
fn papertrade_customizations_use_generated_surfaces() {
    for field in [
        "background",
        "text",
        "primary",
        "success",
        "warning",
        "danger",
    ] {
        let mut config = KeroseneConfig::default();
        let papertrade = config
            .custom_themes
            .iter_mut()
            .find(|theme| theme.name == "Papertrade")
            .expect("Papertrade preset");
        let mut value = serde_json::to_value(&*papertrade).expect("theme serializes");
        value[field] = serde_json::json!("#123456");
        *papertrade = serde_json::from_value(value).expect("customized theme deserializes");

        let (terminal, _) = TradingTerminal::boot_from_config(config);
        let theme = terminal.get_theme_by_name("Custom: Papertrade");

        assert!(
            !TradingTerminal::palette_matches_papertrade_source(theme.palette()),
            "customized {field} must disable the source palette"
        );
        assert_ne!(
            theme.extended_palette().background.weak,
            TradingTerminal::papertrade_source_extended_palette()
                .background
                .weak,
            "customized {field} must use generated surfaces"
        );
    }
}
