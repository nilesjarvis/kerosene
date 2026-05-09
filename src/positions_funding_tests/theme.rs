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
