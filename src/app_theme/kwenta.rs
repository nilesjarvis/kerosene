use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_kwenta_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x13, 0x12, 0x12])
            && super::rgba8_eq(palette.text, [0xF4, 0xF1, 0xE8])
            && super::rgba8_eq(palette.primary, [0xFE, 0xB7, 0x00])
            && super::rgba8_eq(palette.success, [0x71, 0xD2, 0x7A])
            && super::rgba8_eq(palette.warning, [0xFE, 0xB7, 0x00])
            && super::rgba8_eq(palette.danger, [0xF0, 0x50, 0x50])
    }

    pub(crate) fn kwenta_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let bg = color(0x13, 0x12, 0x12);
        let panel_low = color(0x16, 0x15, 0x15);
        let panel = color(0x1A, 0x19, 0x19);
        let panel_high = color(0x25, 0x25, 0x25);
        let panel_active = color(0x2C, 0x2F, 0x2D);
        let text = color(0xF4, 0xF1, 0xE8);
        let text_muted = color(0x8E, 0x89, 0x84);
        let text_dim = color(0x5A, 0x56, 0x53);
        let gold = color(0xFE, 0xB7, 0x00);
        let gold_soft = color(0xFF, 0xD6, 0x4A);
        let gold_dark = color(0x33, 0x28, 0x10);
        let green = color(0x71, 0xD2, 0x7A);
        let green_bright = color(0x83, 0xF0, 0x8D);
        let green_dark = color(0x13, 0x2A, 0x1B);
        let red = color(0xF0, 0x50, 0x50);
        let red_bright = color(0xFF, 0x69, 0x65);
        let red_dark = color(0x34, 0x19, 0x1A);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(color(0x0D, 0x0B, 0x0C), text_dim),
                weaker: super::pair(panel_low, text_dim),
                weak: super::pair(panel, text_muted),
                neutral: super::pair(color(0x1F, 0x1E, 0x1E), text),
                strong: super::pair(panel_high, text),
                stronger: super::pair(panel_active, text),
                strongest: super::pair(color(0x36, 0x34, 0x34), text),
            },
            primary: Primary {
                base: super::pair(gold, bg),
                weak: super::pair(gold_dark, gold),
                strong: super::pair(gold_soft, bg),
            },
            secondary: Secondary {
                base: super::pair(text_muted, bg),
                weak: super::pair(text_dim, bg),
                strong: super::pair(text, bg),
            },
            success: Success {
                base: super::pair(green, bg),
                weak: super::pair(green_dark, green_bright),
                strong: super::pair(green_bright, bg),
            },
            warning: Warning {
                base: super::pair(gold, bg),
                weak: super::pair(gold_dark, gold),
                strong: super::pair(gold_soft, bg),
            },
            danger: Danger {
                base: super::pair(red, bg),
                weak: super::pair(red_dark, red_bright),
                strong: super::pair(red_bright, bg),
            },
            is_dark: true,
        }
    }
}
