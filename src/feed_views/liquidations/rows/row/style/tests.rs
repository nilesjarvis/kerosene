use super::liquidation_row_style;
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use iced::{Background, Color};

#[test]
fn liquidation_sides_use_theme_candle_colors_without_overrides() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());

    for name in ["Dark", "Light"] {
        terminal.active_theme = name.to_string();
        let theme = terminal.theme();
        for (is_buy, expected) in [
            (true, theme.palette().success),
            (false, theme.palette().danger),
        ] {
            let (color, _) = terminal.liquidation_row_color(&theme, is_buy, 10_000.0);
            assert_eq!(color, expected, "{name}, is_buy={is_buy}");
        }
    }
}

#[test]
fn liquidation_sides_keep_custom_candle_hues_at_every_notional_tier() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let custom_theme = terminal.custom_themes.first_mut().expect("default theme");
    custom_theme.chart_bull = Some("#3366CC".to_string());
    custom_theme.chart_bear = Some("#CC6600".to_string());
    terminal.active_theme = format!("Custom: {}", custom_theme.name);
    let theme = terminal.theme();

    for (is_buy, expected) in [
        (true, Color::from_rgb8(0x33, 0x66, 0xCC)),
        (false, Color::from_rgb8(0xCC, 0x66, 0x00)),
    ] {
        for (notional, expected_opacity) in [
            (999.0, 0.02),
            (1_000.0, 0.05),
            (10_000.0, 0.1),
            (50_000.0, 0.2),
            (100_000.0, 0.35),
            (500_000.0, 0.6),
        ] {
            let (color, opacity) = terminal.liquidation_row_color(&theme, is_buy, notional);
            assert_eq!(color, expected);
            assert_eq!(opacity, expected_opacity);
            assert_eq!(
                liquidation_row_style(color, opacity, 4.0).background,
                Some(Background::Color(Color {
                    a: expected_opacity,
                    ..expected
                }))
            );
        }
    }
}
