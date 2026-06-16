use crate::app_state::TradingTerminal;
use iced::Color;

fn assert_rgba(color: Color, expected: [u8; 4]) {
    assert_eq!(color.into_rgba8(), expected);
}

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
    assert_rgba(extended.background.base.color, [0x0F, 0x1A, 0x1E, 255]);
    assert_rgba(extended.background.weak.color, [0x1B, 0x24, 0x29, 255]);
    assert_rgba(extended.background.strong.color, [0x27, 0x30, 0x35, 255]);
    assert_rgba(extended.background.weak.text, [0x94, 0x9E, 0x9C, 255]);
    assert_rgba(extended.primary.base.color, [0x50, 0xD2, 0xC1, 255]);
    assert_rgba(extended.success.base.color, [0x50, 0xD2, 0xC1, 255]);
    assert_rgba(extended.danger.base.color, [0xED, 0x70, 0x88, 255]);
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
    assert_rgba(extended.background.base.color, [0x00, 0x00, 0x00, 255]);
    assert_rgba(extended.background.weak.color, [0x00, 0x00, 0x00, 255]);
    assert_rgba(extended.background.strong.color, [0x00, 0x00, 0x00, 255]);
    assert_rgba(extended.background.stronger.color, [0x05, 0x05, 0x05, 255]);
    assert_rgba(extended.primary.base.color, [0xFF, 0x9F, 0x1A, 255]);
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
    assert_rgba(extended.background.base.color, [0x0B, 0x07, 0x11, 255]);
    assert_rgba(extended.background.weak.color, [0x17, 0x13, 0x1D, 255]);
    assert_rgba(extended.background.strong.color, [0x21, 0x1D, 0x28, 255]);
    assert_rgba(extended.primary.base.color, [0x71, 0x32, 0xF5, 255]);
    assert_rgba(extended.success.base.color, [0x2B, 0xB6, 0x7B, 255]);
    assert_rgba(extended.danger.strong.color, [0xE3, 0x4A, 0x6F, 255]);
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
    assert_rgba(extended.background.base.color, [0x10, 0x18, 0x24, 255]);
    assert_rgba(extended.background.weak.color, [0x12, 0x1A, 0x27, 255]);
    assert_rgba(extended.background.strong.color, [0x18, 0x22, 0x32, 255]);
    assert_rgba(extended.primary.base.color, [0x00, 0xA8, 0xB8, 255]);
    assert_rgba(extended.success.base.color, [0x08, 0xA6, 0x7A, 255]);
    assert_rgba(extended.danger.base.color, [0xF0, 0x30, 0x60, 255]);
}

#[test]
fn ibkr_dark_theme_uses_tws_blue_panels_and_trade_accents() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x10, 0x10, 0x18),
        text: Color::from_rgb8(0xD8, 0xDC, 0xE6),
        primary: Color::from_rgb8(0x28, 0x78, 0xF0),
        success: Color::from_rgb8(0x2E, 0xBF, 0x7A),
        warning: Color::from_rgb8(0xD0, 0xA8, 0x18),
        danger: Color::from_rgb8(0xF8, 0x30, 0x48),
    };

    assert!(TradingTerminal::palette_matches_ibkr_dark_source(
        source_palette
    ));

    let extended = TradingTerminal::ibkr_dark_source_extended_palette();
    assert_rgba(extended.background.base.color, [0x10, 0x10, 0x18, 255]);
    assert_rgba(extended.background.weak.color, [0x15, 0x17, 0x24, 255]);
    assert_rgba(extended.background.strong.color, [0x20, 0x28, 0x38, 255]);
    assert_rgba(extended.primary.base.color, [0x28, 0x78, 0xF0, 255]);
    assert_rgba(extended.success.base.color, [0x2E, 0xBF, 0x7A, 255]);
    assert_rgba(extended.danger.base.color, [0xF8, 0x30, 0x48, 255]);
}

#[test]
fn bybit_theme_uses_charcoal_panels_and_amber_actions() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x10, 0x10, 0x14),
        text: Color::from_rgb8(0xF5, 0xF5, 0xF5),
        primary: Color::from_rgb8(0xF4, 0xB4, 0x44),
        success: Color::from_rgb8(0x55, 0xAF, 0x72),
        warning: Color::from_rgb8(0xE8, 0xA8, 0x38),
        danger: Color::from_rgb8(0xDC, 0x53, 0x51),
    };

    assert!(TradingTerminal::palette_matches_bybit_source(
        source_palette
    ));

    let extended = TradingTerminal::bybit_source_extended_palette();
    assert_rgba(extended.background.base.color, [0x10, 0x10, 0x14, 255]);
    assert_rgba(extended.background.weak.color, [0x17, 0x18, 0x1D, 255]);
    assert_rgba(extended.background.strong.color, [0x20, 0x21, 0x24, 255]);
    assert_rgba(extended.primary.base.color, [0xF4, 0xB4, 0x44, 255]);
    assert_rgba(extended.success.base.color, [0x55, 0xAF, 0x72, 255]);
    assert_rgba(extended.danger.base.color, [0xDC, 0x53, 0x51, 255]);
}

#[test]
fn kwenta_theme_uses_screenshot_black_panels_and_gold_actions() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x13, 0x12, 0x12),
        text: Color::from_rgb8(0xF4, 0xF1, 0xE8),
        primary: Color::from_rgb8(0xFE, 0xB7, 0x00),
        success: Color::from_rgb8(0x71, 0xD2, 0x7A),
        warning: Color::from_rgb8(0xFE, 0xB7, 0x00),
        danger: Color::from_rgb8(0xF0, 0x50, 0x50),
    };

    assert!(TradingTerminal::palette_matches_kwenta_source(
        source_palette
    ));

    let extended = TradingTerminal::kwenta_source_extended_palette();
    assert_rgba(extended.background.base.color, [0x13, 0x12, 0x12, 255]);
    assert_rgba(extended.background.weak.color, [0x1A, 0x19, 0x19, 255]);
    assert_rgba(extended.background.strong.color, [0x25, 0x25, 0x25, 255]);
    assert_rgba(extended.primary.base.color, [0xFE, 0xB7, 0x00, 255]);
    assert_rgba(extended.success.base.color, [0x71, 0xD2, 0x7A, 255]);
    assert_rgba(extended.danger.base.color, [0xF0, 0x50, 0x50, 255]);
    assert!(extended.is_dark);
}

#[test]
fn coinbase_dark_theme_uses_sampled_black_panels_and_blue_actions() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x09, 0x0B, 0x0C),
        text: Color::from_rgb8(0xF5, 0xF7, 0xF9),
        primary: Color::from_rgb8(0x34, 0x74, 0xF4),
        success: Color::from_rgb8(0x44, 0xC4, 0x8C),
        warning: Color::from_rgb8(0xF4, 0x94, 0x1C),
        danger: Color::from_rgb8(0xEC, 0x64, 0x74),
    };

    assert!(TradingTerminal::palette_matches_coinbase_dark_source(
        source_palette
    ));

    let extended = TradingTerminal::coinbase_dark_source_extended_palette();
    assert_rgba(extended.background.base.color, [0x09, 0x0B, 0x0C, 255]);
    assert_rgba(extended.background.weak.color, [0x13, 0x15, 0x19, 255]);
    assert_rgba(extended.background.strong.color, [0x24, 0x26, 0x2D, 255]);
    assert_rgba(extended.primary.base.color, [0x34, 0x74, 0xF4, 255]);
    assert_rgba(extended.success.base.color, [0x44, 0xC4, 0x8C, 255]);
    assert_rgba(extended.danger.base.color, [0xEC, 0x64, 0x74, 255]);
    assert!(extended.is_dark);
}

#[test]
fn coinbase_light_theme_uses_clean_portfolio_surfaces_and_blue_actions() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0xFF, 0xFF, 0xFF),
        text: Color::from_rgb8(0x0A, 0x0B, 0x0D),
        primary: Color::from_rgb8(0x00, 0x52, 0xFF),
        success: Color::from_rgb8(0x09, 0x85, 0x51),
        warning: Color::from_rgb8(0xF7, 0x93, 0x1A),
        danger: Color::from_rgb8(0xCF, 0x20, 0x2F),
    };

    assert!(TradingTerminal::palette_matches_coinbase_light_source(
        source_palette
    ));

    let extended = TradingTerminal::coinbase_light_source_extended_palette();
    assert_rgba(extended.background.base.color, [0xFF, 0xFF, 0xFF, 255]);
    assert_rgba(extended.background.weaker.color, [0xF5, 0xF8, 0xFF, 255]);
    assert_rgba(extended.background.strong.color, [0xE0, 0xE4, 0xEA, 255]);
    assert_rgba(extended.primary.base.color, [0x00, 0x52, 0xFF, 255]);
    assert_rgba(extended.success.base.color, [0x09, 0x85, 0x51, 255]);
    assert!(!extended.is_dark);
}

#[test]
fn ubuntu_theme_uses_aubergine_panels_and_orange_actions() {
    let source_palette = iced::theme::Palette {
        background: Color::from_rgb8(0x1B, 0x0E, 0x18),
        text: Color::from_rgb8(0xF3, 0xEA, 0xEF),
        primary: Color::from_rgb8(0xF6, 0x6D, 0x2C),
        success: Color::from_rgb8(0x33, 0xD1, 0x7A),
        warning: Color::from_rgb8(0xFF, 0xD2, 0x4A),
        danger: Color::from_rgb8(0xF5, 0x46, 0x5F),
    };

    assert!(TradingTerminal::palette_matches_ubuntu_source(
        source_palette
    ));

    let extended = TradingTerminal::ubuntu_source_extended_palette();
    // Aubergine ladder steps up monotonically from the base.
    assert_rgba(extended.background.base.color, [0x1B, 0x0E, 0x18, 255]);
    assert_rgba(extended.background.weak.color, [0x2C, 0x1B, 0x27, 255]);
    assert_rgba(extended.background.strong.color, [0x41, 0x2B, 0x3D, 255]);
    assert_rgba(extended.background.strongest.color, [0x62, 0x47, 0x5D, 255]);
    // Ubuntu Orange hero plus the separated gold/green/rose semantics.
    assert_rgba(extended.primary.base.color, [0xF6, 0x6D, 0x2C, 255]);
    assert_rgba(extended.success.base.color, [0x33, 0xD1, 0x7A, 255]);
    assert_rgba(extended.warning.base.color, [0xFF, 0xD2, 0x4A, 255]);
    assert_rgba(extended.danger.base.color, [0xF5, 0x46, 0x5F, 255]);
    assert!(extended.is_dark);
}
