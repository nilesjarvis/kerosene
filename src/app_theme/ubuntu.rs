use crate::app_state::TradingTerminal;
use iced::Color;

fn rgba8_eq(color: Color, rgb: [u8; 3]) -> bool {
    color.into_rgba8() == [rgb[0], rgb[1], rgb[2], 255]
}

impl TradingTerminal {
    pub(crate) fn palette_matches_ubuntu_source(palette: iced::theme::Palette) -> bool {
        (rgba8_eq(palette.background, [0x56, 0x33, 0x4B])
            || rgba8_eq(palette.background, [0x2C, 0x00, 0x1E]))
            && rgba8_eq(palette.text, [0xF6, 0xF6, 0xF5])
            && rgba8_eq(palette.primary, [0xE9, 0x54, 0x20])
            && rgba8_eq(palette.success, [0x2E, 0xC2, 0x7E])
            && rgba8_eq(palette.warning, [0xF9, 0x9B, 0x11])
            && rgba8_eq(palette.danger, [0xC7, 0x16, 0x2B])
    }

    pub(crate) fn ubuntu_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let pair = |color, text| Pair { color, text };

        let aubergine_bg = color(0x56, 0x33, 0x4B);
        let aubergine_bg_hover = color(0x6B, 0x4C, 0x61);
        let dark_aubergine = color(0x2C, 0x00, 0x1E);
        let panel_low = color(0x21, 0x0A, 0x1C);
        let panel_deep = color(0x1A, 0x06, 0x15);
        let panel_deeper = color(0x12, 0x04, 0x0F);
        let panel = color(0x36, 0x12, 0x2C);
        let panel_high = color(0x41, 0x19, 0x34);
        let text = color(0xF6, 0xF6, 0xF5);
        let text_soft = color(0xEE, 0xED, 0xEB);
        let text_muted = color(0xAE, 0xA7, 0x9F);
        let text_dim = color(0x80, 0x66, 0x78);
        let orange = color(0xE9, 0x54, 0x20);
        let orange_bright = color(0xF0, 0x87, 0x63);
        let orange_dark = color(0x47, 0x18, 0x0C);
        let green = color(0x2E, 0xC2, 0x7E);
        let green_bright = color(0x57, 0xE3, 0x9B);
        let green_dark = color(0x0F, 0x32, 0x28);
        let amber = color(0xF9, 0x9B, 0x11);
        let amber_bright = color(0xF6, 0xBB, 0xA6);
        let amber_dark = color(0x3A, 0x24, 0x0A);
        let red = color(0xC7, 0x16, 0x2B);
        let red_bright = color(0xED, 0x76, 0x4D);
        let red_dark = color(0x3B, 0x12, 0x1A);

        Extended {
            background: Background {
                base: pair(aubergine_bg, text),
                weakest: pair(aubergine_bg_hover, text_dim),
                weaker: pair(panel_high, text_dim),
                weak: pair(panel, text_muted),
                neutral: pair(dark_aubergine, text_soft),
                strong: pair(panel_low, text),
                stronger: pair(panel_deep, text),
                strongest: pair(panel_deeper, text),
            },
            primary: Primary {
                base: pair(orange, text),
                weak: pair(orange_dark, orange_bright),
                strong: pair(orange_bright, dark_aubergine),
            },
            secondary: Secondary {
                base: pair(text_muted, dark_aubergine),
                weak: pair(text_dim, dark_aubergine),
                strong: pair(text, dark_aubergine),
            },
            success: Success {
                base: pair(green, dark_aubergine),
                weak: pair(green_dark, green_bright),
                strong: pair(green_bright, dark_aubergine),
            },
            warning: Warning {
                base: pair(amber, dark_aubergine),
                weak: pair(amber_dark, amber_bright),
                strong: pair(amber_bright, dark_aubergine),
            },
            danger: Danger {
                base: pair(red, text),
                weak: pair(red_dark, red_bright),
                strong: pair(red_bright, dark_aubergine),
            },
            is_dark: true,
        }
    }
}
