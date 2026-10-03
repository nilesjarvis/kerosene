use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use iced::Color;

#[test]
fn tradingview_uses_reference_surfaces_and_chart_colors() {
    let (terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig {
        active_theme: "Custom: TradingView".to_string(),
        ..KeroseneConfig::default()
    });
    let theme = terminal.theme();
    let palette = theme.palette();
    let extended = theme.extended_palette();

    assert!(TradingTerminal::palette_matches_tradingview_source(palette));
    assert_eq!(extended.background.base.color, palette.background);
    assert_eq!(extended.background.strong.color, palette.background);
    assert_eq!(
        extended.background.weak.color,
        Color::from_rgb8(0x1E, 0x22, 0x2D)
    );
    assert_eq!(
        extended.background.weak.text,
        Color::from_rgb8(0xB2, 0xB5, 0xBE)
    );
    assert_eq!(extended.primary.base.color, palette.primary);
    assert_eq!(extended.primary.base.text, Color::WHITE);
    assert_eq!(extended.success.base.color, palette.success);
    assert_eq!(extended.warning.base.color, palette.warning);
    assert_eq!(extended.danger.base.color, palette.danger);
    assert!(extended.is_dark);

    let overrides = terminal.active_chart_theme_overrides();
    assert_eq!(overrides.bull, Some(Color::from_rgb8(0x26, 0xA6, 0x9A)));
    assert_eq!(overrides.bear, Some(Color::from_rgb8(0xEF, 0x53, 0x50)));
    assert_eq!(overrides.line, Some(Color::from_rgb8(0x29, 0x62, 0xFF)));
    assert_eq!(overrides.line_gradient, None);
    assert_eq!(
        terminal.direction_colors(&theme),
        (palette.success, palette.danger)
    );
}

#[test]
fn tradingview_customizations_use_generated_surfaces() {
    for field in [
        "background",
        "text",
        "primary",
        "success",
        "warning",
        "danger",
    ] {
        let mut config = KeroseneConfig::default();
        let tradingview = config
            .custom_themes
            .iter_mut()
            .find(|theme| theme.name == "TradingView")
            .expect("TradingView preset");
        let mut value = serde_json::to_value(&*tradingview).expect("theme serializes");
        value[field] = serde_json::json!("#123456");
        *tradingview = serde_json::from_value(value).expect("customized theme deserializes");

        let (terminal, _) = TradingTerminal::boot_from_config(config);
        let theme = terminal.get_theme_by_name("Custom: TradingView");

        assert!(
            !TradingTerminal::palette_matches_tradingview_source(theme.palette()),
            "customized {field} must disable the source palette"
        );
        assert_ne!(
            theme.extended_palette().background.weak,
            TradingTerminal::tradingview_source_extended_palette()
                .background
                .weak,
            "customized {field} must use generated surfaces"
        );
    }
}
