use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_bloomberg_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x00, 0x00, 0x00])
            && super::rgba8_eq(palette.text, [0xF2, 0xF2, 0xE8])
            && super::rgba8_eq(palette.primary, [0xFF, 0x9F, 0x1A])
            && super::rgba8_eq(palette.success, [0x00, 0xB0, 0x50])
            && super::rgba8_eq(palette.warning, [0xFF, 0xD8, 0x4A])
            && super::rgba8_eq(palette.danger, [0xB0, 0x00, 0x24])
    }

    pub(crate) fn bloomberg_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let black = color(0x00, 0x00, 0x00);
        let panel_black = color(0x05, 0x05, 0x05);
        let panel_black_high = color(0x0A, 0x0A, 0x0A);
        let text = color(0xF2, 0xF2, 0xE8);
        let text_muted = color(0x8C, 0x8C, 0x80);
        let text_dim = color(0x58, 0x58, 0x52);
        let amber = color(0xFF, 0x9F, 0x1A);
        let amber_soft = color(0xFF, 0xD8, 0x4A);
        let amber_dark = color(0x33, 0x1D, 0x00);
        let green = color(0x00, 0xB0, 0x50);
        let green_bright = color(0x00, 0xC8, 0x53);
        let green_dark = color(0x00, 0x25, 0x12);
        let red = color(0xB0, 0x00, 0x24);
        let red_bright = color(0xD5, 0x00, 0x32);
        let red_dark = color(0x2A, 0x00, 0x09);

        Extended {
            background: Background {
                base: super::pair(black, text),
                weakest: super::pair(black, text_dim),
                weaker: super::pair(black, text_muted),
                weak: super::pair(black, text_muted),
                neutral: super::pair(black, text),
                strong: super::pair(black, text),
                stronger: super::pair(panel_black, text),
                strongest: super::pair(panel_black_high, text),
            },
            primary: Primary {
                base: super::pair(amber, black),
                weak: super::pair(amber_dark, amber),
                strong: super::pair(amber_soft, black),
            },
            secondary: Secondary {
                base: super::pair(text_muted, black),
                weak: super::pair(text_dim, black),
                strong: super::pair(text, black),
            },
            success: Success {
                base: super::pair(green, black),
                weak: super::pair(green_dark, green_bright),
                strong: super::pair(green_bright, black),
            },
            warning: Warning {
                base: super::pair(amber_soft, black),
                weak: super::pair(amber_dark, amber_soft),
                strong: super::pair(color(0xFF, 0xE6, 0x6D), black),
            },
            danger: Danger {
                base: super::pair(red, text),
                weak: super::pair(red_dark, red_bright),
                strong: super::pair(red_bright, text),
            },
            is_dark: true,
        }
    }
}
