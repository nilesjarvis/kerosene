use crate::app_state::TradingTerminal;
use iced::Color;

fn rgba8_eq(color: Color, rgb: [u8; 3]) -> bool {
    color.into_rgba8() == [rgb[0], rgb[1], rgb[2], 255]
}

impl TradingTerminal {
    pub(crate) fn palette_matches_coinbase_dark_source(palette: iced::theme::Palette) -> bool {
        rgba8_eq(palette.background, [0x09, 0x0B, 0x0C])
            && rgba8_eq(palette.text, [0xF5, 0xF7, 0xF9])
            && rgba8_eq(palette.primary, [0x34, 0x74, 0xF4])
            && rgba8_eq(palette.success, [0x44, 0xC4, 0x8C])
            && rgba8_eq(palette.warning, [0xF4, 0x94, 0x1C])
            && rgba8_eq(palette.danger, [0xEC, 0x64, 0x74])
    }

    pub(crate) fn coinbase_dark_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let pair = |color, text| Pair { color, text };

        let bg = color(0x09, 0x0B, 0x0C);
        let panel_low = color(0x05, 0x08, 0x09);
        let panel_subtle = color(0x0A, 0x0D, 0x0F);
        let panel = color(0x13, 0x15, 0x19);
        let panel_high = color(0x19, 0x1B, 0x20);
        let border = color(0x24, 0x26, 0x2D);
        let control = color(0x2E, 0x30, 0x36);
        let control_hover = color(0x3A, 0x3D, 0x46);
        let text = color(0xF5, 0xF7, 0xF9);
        let text_soft = color(0xC4, 0xC7, 0xCF);
        let text_muted = color(0x8A, 0x8F, 0x98);
        let text_dim = color(0x5F, 0x64, 0x6D);
        let blue = color(0x34, 0x74, 0xF4);
        let blue_bright = color(0x5C, 0x8C, 0xF4);
        let blue_dark = color(0x0D, 0x24, 0x55);
        let green = color(0x44, 0xC4, 0x8C);
        let green_bright = color(0x5B, 0xE0, 0xA4);
        let green_dark = color(0x0F, 0x2C, 0x22);
        let orange = color(0xF4, 0x94, 0x1C);
        let orange_bright = color(0xEC, 0xD4, 0x6C);
        let orange_dark = color(0x33, 0x22, 0x10);
        let red = color(0xEC, 0x64, 0x74);
        let red_bright = color(0xF8, 0x74, 0x84);
        let red_dark = color(0x35, 0x1C, 0x23);

        Extended {
            background: Background {
                base: pair(bg, text),
                weakest: pair(panel_low, text_dim),
                weaker: pair(panel_subtle, text_dim),
                weak: pair(panel, text_muted),
                neutral: pair(panel_high, text_soft),
                strong: pair(border, text),
                stronger: pair(control, text),
                strongest: pair(control_hover, text),
            },
            primary: Primary {
                base: pair(blue, bg),
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
                base: pair(orange, bg),
                weak: pair(orange_dark, orange),
                strong: pair(orange_bright, bg),
            },
            danger: Danger {
                base: pair(red, bg),
                weak: pair(red_dark, red),
                strong: pair(red_bright, bg),
            },
            is_dark: true,
        }
    }
}
