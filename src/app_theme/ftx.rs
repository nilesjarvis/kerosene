use crate::app_state::TradingTerminal;
use iced::Color;

fn rgba8_eq(color: Color, rgb: [u8; 3]) -> bool {
    color.into_rgba8() == [rgb[0], rgb[1], rgb[2], 255]
}

impl TradingTerminal {
    pub(crate) fn palette_matches_ftx_source(palette: iced::theme::Palette) -> bool {
        rgba8_eq(palette.background, [0x10, 0x18, 0x24])
            && rgba8_eq(palette.text, [0xD8, 0xE2, 0xEE])
            && rgba8_eq(palette.primary, [0x00, 0xA8, 0xB8])
            && rgba8_eq(palette.success, [0x08, 0xA6, 0x7A])
            && rgba8_eq(palette.warning, [0xF0, 0xA0, 0x40])
            && rgba8_eq(palette.danger, [0xF0, 0x30, 0x60])
    }

    pub(crate) fn ftx_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let pair = |color, text| Pair { color, text };

        let bg = color(0x10, 0x18, 0x24);
        let panel_low = color(0x0C, 0x12, 0x1C);
        let panel = color(0x12, 0x1A, 0x27);
        let panel_high = color(0x18, 0x22, 0x32);
        let panel_active = color(0x21, 0x2D, 0x3D);
        let text = color(0xD8, 0xE2, 0xEE);
        let text_muted = color(0x7A, 0x86, 0x96);
        let text_dim = color(0x50, 0x5B, 0x68);
        let cyan = color(0x00, 0xA8, 0xB8);
        let cyan_bright = color(0x4B, 0xD0, 0xDF);
        let cyan_dark = color(0x07, 0x34, 0x3D);
        let green = color(0x08, 0xA6, 0x7A);
        let green_bright = color(0x18, 0xC9, 0x94);
        let green_dark = color(0x06, 0x34, 0x29);
        let amber = color(0xF0, 0xA0, 0x40);
        let amber_dark = color(0x3C, 0x2A, 0x18);
        let red = color(0xF0, 0x30, 0x60);
        let red_dark = color(0x3A, 0x17, 0x25);

        Extended {
            background: Background {
                base: pair(bg, text),
                weakest: pair(panel_low, text_dim),
                weaker: pair(color(0x0F, 0x16, 0x21), text_dim),
                weak: pair(panel, text_muted),
                neutral: pair(color(0x15, 0x1F, 0x2E), text),
                strong: pair(panel_high, text),
                stronger: pair(panel_active, text),
                strongest: pair(color(0x2B, 0x38, 0x49), text),
            },
            primary: Primary {
                base: pair(cyan, bg),
                weak: pair(cyan_dark, cyan_bright),
                strong: pair(cyan_bright, bg),
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
                strong: pair(color(0xFF, 0xC1, 0x6B), bg),
            },
            danger: Danger {
                base: pair(red, text),
                weak: pair(red_dark, red),
                strong: pair(color(0xFF, 0x55, 0x7C), text),
            },
            is_dark: true,
        }
    }
}
