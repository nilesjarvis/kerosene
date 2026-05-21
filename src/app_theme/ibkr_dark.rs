use crate::app_state::TradingTerminal;
use iced::Color;

fn rgba8_eq(color: Color, rgb: [u8; 3]) -> bool {
    color.into_rgba8() == [rgb[0], rgb[1], rgb[2], 255]
}

impl TradingTerminal {
    pub(crate) fn palette_matches_ibkr_dark_source(palette: iced::theme::Palette) -> bool {
        rgba8_eq(palette.background, [0x10, 0x10, 0x18])
            && rgba8_eq(palette.text, [0xD8, 0xDC, 0xE6])
            && rgba8_eq(palette.primary, [0x28, 0x78, 0xF0])
            && rgba8_eq(palette.success, [0x2E, 0xBF, 0x7A])
            && rgba8_eq(palette.warning, [0xD0, 0xA8, 0x18])
            && rgba8_eq(palette.danger, [0xF8, 0x30, 0x48])
    }

    pub(crate) fn ibkr_dark_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let pair = |color, text| Pair { color, text };

        let bg = color(0x10, 0x10, 0x18);
        let panel_low = color(0x0B, 0x0C, 0x14);
        let panel = color(0x15, 0x17, 0x24);
        let panel_high = color(0x20, 0x28, 0x38);
        let panel_active = color(0x2B, 0x30, 0x40);
        let text = color(0xD8, 0xDC, 0xE6);
        let text_muted = color(0x8A, 0x91, 0xA4);
        let text_dim = color(0x5E, 0x65, 0x77);
        let blue = color(0x28, 0x78, 0xF0);
        let blue_bright = color(0x5A, 0xA2, 0xFF);
        let blue_dark = color(0x08, 0x20, 0x48);
        let green = color(0x2E, 0xBF, 0x7A);
        let green_bright = color(0x4A, 0xD0, 0x91);
        let green_dark = color(0x10, 0x30, 0x28);
        let amber = color(0xD0, 0xA8, 0x18);
        let amber_dark = color(0x30, 0x28, 0x17);
        let red = color(0xF8, 0x30, 0x48);
        let red_dark = color(0x3A, 0x17, 0x20);

        Extended {
            background: Background {
                base: pair(bg, text),
                weakest: pair(panel_low, text_dim),
                weaker: pair(color(0x0D, 0x0E, 0x17), text_dim),
                weak: pair(panel, text_muted),
                neutral: pair(color(0x1B, 0x1E, 0x2B), text),
                strong: pair(panel_high, text),
                stronger: pair(panel_active, text),
                strongest: pair(color(0x38, 0x40, 0x58), text),
            },
            primary: Primary {
                base: pair(blue, text),
                weak: pair(blue_dark, blue_bright),
                strong: pair(blue_bright, bg),
            },
            secondary: Secondary {
                base: pair(text_muted, bg),
                weak: pair(text_dim, bg),
                strong: pair(text, bg),
            },
            success: Success {
                base: pair(green, bg),
                weak: pair(green_dark, green_bright),
                strong: pair(green_bright, bg),
            },
            warning: Warning {
                base: pair(amber, bg),
                weak: pair(amber_dark, amber),
                strong: pair(color(0xF0, 0xC8, 0x38), bg),
            },
            danger: Danger {
                base: pair(red, text),
                weak: pair(red_dark, red),
                strong: pair(color(0xFF, 0x58, 0x66), text),
            },
            is_dark: true,
        }
    }
}
