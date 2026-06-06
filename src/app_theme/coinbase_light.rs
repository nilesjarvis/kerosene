use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_coinbase_light_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0xFF, 0xFF, 0xFF])
            && super::rgba8_eq(palette.text, [0x0A, 0x0B, 0x0D])
            && super::rgba8_eq(palette.primary, [0x00, 0x52, 0xFF])
            && super::rgba8_eq(palette.success, [0x09, 0x85, 0x51])
            && super::rgba8_eq(palette.warning, [0xF7, 0x93, 0x1A])
            && super::rgba8_eq(palette.danger, [0xCF, 0x20, 0x2F])
    }

    pub(crate) fn coinbase_light_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let bg = color(0xFF, 0xFF, 0xFF);
        let nav_selected = color(0xF5, 0xF8, 0xFF);
        let panel = color(0xF8, 0xFA, 0xFD);
        let border = color(0xE0, 0xE4, 0xEA);
        let control = color(0xF2, 0xF4, 0xF7);
        let control_hover = color(0xEA, 0xEF, 0xF6);
        let text = color(0x0A, 0x0B, 0x0D);
        let text_muted = color(0x6B, 0x72, 0x80);
        let text_dim = color(0x98, 0xA1, 0xAE);
        let blue = color(0x00, 0x52, 0xFF);
        let blue_bright = color(0x0A, 0x66, 0xFF);
        let blue_soft = color(0xEA, 0xF3, 0xFF);
        let green = color(0x09, 0x85, 0x51);
        let green_soft = color(0xE7, 0xF6, 0xEF);
        let green_bright = color(0x0A, 0xA6, 0x5D);
        let orange = color(0xF7, 0x93, 0x1A);
        let orange_soft = color(0xFF, 0xF2, 0xDF);
        let red = color(0xCF, 0x20, 0x2F);
        let red_soft = color(0xFE, 0xEA, 0xED);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(bg, text_dim),
                weaker: super::pair(nav_selected, text_muted),
                weak: super::pair(panel, text_muted),
                neutral: super::pair(control, text),
                strong: super::pair(border, text),
                stronger: super::pair(control_hover, text),
                strongest: super::pair(color(0xD3, 0xDA, 0xE5), text),
            },
            primary: Primary {
                base: super::pair(blue, bg),
                weak: super::pair(blue_soft, blue),
                strong: super::pair(blue_bright, bg),
            },
            secondary: Secondary {
                base: super::pair(text_muted, bg),
                weak: super::pair(text_dim, bg),
                strong: super::pair(text, bg),
            },
            success: Success {
                base: super::pair(green, bg),
                weak: super::pair(green_soft, green),
                strong: super::pair(green_bright, bg),
            },
            warning: Warning {
                base: super::pair(orange, bg),
                weak: super::pair(orange_soft, orange),
                strong: super::pair(color(0xD8, 0x73, 0x00), bg),
            },
            danger: Danger {
                base: super::pair(red, bg),
                weak: super::pair(red_soft, red),
                strong: super::pair(color(0xA8, 0x14, 0x24), bg),
            },
            is_dark: false,
        }
    }
}
