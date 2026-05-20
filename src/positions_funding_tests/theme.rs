use crate::app_state::TradingTerminal;
use iced::Color;

#[test]
fn hyperliquid_theme_uses_source_palette_after_extended_processing() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x0F, 0x1A, 0x1E),
        text: Color::from_rgb8(0xF6, 0xFE, 0xFD),
        primary: Color::from_rgb8(0x50, 0xD2, 0xC1),
        success: Color::from_rgb8(0x50, 0xD2, 0xC1),
        warning: Color::from_rgb8(0xFF, 0xB6, 0x48),
        danger: Color::from_rgb8(0xED, 0x70, 0x88),
    };

    assert!(TradingTerminal::palette_matches_hyperliquid_source(
        source_palette
    ));

    let extended = TradingTerminal::hyperliquid_source_extended_palette();
    assert_eq!(
        extended.background.base.color.into_rgba8(),
        [0x0F, 0x1A, 0x1E, 255]
    );
    assert_eq!(
        extended.background.weak.color.into_rgba8(),
        [0x1B, 0x24, 0x29, 255]
    );
    assert_eq!(
        extended.background.strong.color.into_rgba8(),
        [0x27, 0x30, 0x35, 255]
    );
    assert_eq!(
        extended.background.weak.text.into_rgba8(),
        [0x94, 0x9E, 0x9C, 255]
    );
    assert_eq!(
        extended.primary.base.color.into_rgba8(),
        [0x50, 0xD2, 0xC1, 255]
    );
    assert_eq!(
        extended.success.base.color.into_rgba8(),
        [0x50, 0xD2, 0xC1, 255]
    );
    assert_eq!(
        extended.danger.base.color.into_rgba8(),
        [0xED, 0x70, 0x88, 255]
    );
}

#[test]
fn bloomberg_theme_keeps_primary_backgrounds_true_black() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x00, 0x00, 0x00),
        text: Color::from_rgb8(0xF2, 0xF2, 0xE8),
        primary: Color::from_rgb8(0xFF, 0x9F, 0x1A),
        success: Color::from_rgb8(0x00, 0xB0, 0x50),
        warning: Color::from_rgb8(0xFF, 0xD8, 0x4A),
        danger: Color::from_rgb8(0xB0, 0x00, 0x24),
    };

    assert!(TradingTerminal::palette_matches_bloomberg_source(
        source_palette
    ));

    let extended = TradingTerminal::bloomberg_source_extended_palette();
    assert_eq!(
        extended.background.base.color.into_rgba8(),
        [0x00, 0x00, 0x00, 255]
    );
    assert_eq!(
        extended.background.weak.color.into_rgba8(),
        [0x00, 0x00, 0x00, 255]
    );
    assert_eq!(
        extended.background.strong.color.into_rgba8(),
        [0x00, 0x00, 0x00, 255]
    );
    assert_eq!(
        extended.background.stronger.color.into_rgba8(),
        [0x05, 0x05, 0x05, 255]
    );
    assert_eq!(
        extended.primary.base.color.into_rgba8(),
        [0xFF, 0x9F, 0x1A, 255]
    );
}

#[test]
fn kraken_theme_uses_aubergine_panels_and_trade_accents() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x0B, 0x07, 0x11),
        text: Color::from_rgb8(0xE8, 0xE1, 0xF2),
        primary: Color::from_rgb8(0x71, 0x32, 0xF5),
        success: Color::from_rgb8(0x2B, 0xB6, 0x7B),
        warning: Color::from_rgb8(0xED, 0x9B, 0x35),
        danger: Color::from_rgb8(0xB2, 0x42, 0x5F),
    };

    assert!(TradingTerminal::palette_matches_kraken_source(
        source_palette
    ));

    let extended = TradingTerminal::kraken_source_extended_palette();
    assert_eq!(
        extended.background.base.color.into_rgba8(),
        [0x0B, 0x07, 0x11, 255]
    );
    assert_eq!(
        extended.background.weak.color.into_rgba8(),
        [0x17, 0x13, 0x1D, 255]
    );
    assert_eq!(
        extended.background.strong.color.into_rgba8(),
        [0x21, 0x1D, 0x28, 255]
    );
    assert_eq!(
        extended.primary.base.color.into_rgba8(),
        [0x71, 0x32, 0xF5, 255]
    );
    assert_eq!(
        extended.success.base.color.into_rgba8(),
        [0x2B, 0xB6, 0x7B, 255]
    );
    assert_eq!(
        extended.danger.strong.color.into_rgba8(),
        [0xE3, 0x4A, 0x6F, 255]
    );
}

#[test]
fn ftx_theme_uses_screenshot_navy_panels_and_teal_accents() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x10, 0x18, 0x24),
        text: Color::from_rgb8(0xD8, 0xE2, 0xEE),
        primary: Color::from_rgb8(0x00, 0xA8, 0xB8),
        success: Color::from_rgb8(0x08, 0xA6, 0x7A),
        warning: Color::from_rgb8(0xF0, 0xA0, 0x40),
        danger: Color::from_rgb8(0xF0, 0x30, 0x60),
    };

    assert!(TradingTerminal::palette_matches_ftx_source(source_palette));

    let extended = TradingTerminal::ftx_source_extended_palette();
    assert_eq!(
        extended.background.base.color.into_rgba8(),
        [0x10, 0x18, 0x24, 255]
    );
    assert_eq!(
        extended.background.weak.color.into_rgba8(),
        [0x12, 0x1A, 0x27, 255]
    );
    assert_eq!(
        extended.background.strong.color.into_rgba8(),
        [0x18, 0x22, 0x32, 255]
    );
    assert_eq!(
        extended.primary.base.color.into_rgba8(),
        [0x00, 0xA8, 0xB8, 255]
    );
    assert_eq!(
        extended.success.base.color.into_rgba8(),
        [0x08, 0xA6, 0x7A, 255]
    );
    assert_eq!(
        extended.danger.base.color.into_rgba8(),
        [0xF0, 0x30, 0x60, 255]
    );
}
