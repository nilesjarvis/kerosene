use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_ftx_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x10, 0x18, 0x24])
            && super::rgba8_eq(palette.text, [0xD8, 0xE2, 0xEE])
            && super::rgba8_eq(palette.primary, [0x00, 0xA8, 0xB8])
            && super::rgba8_eq(palette.success, [0x08, 0xA6, 0x7A])
            && super::rgba8_eq(palette.warning, [0xF0, 0xA0, 0x40])
            && super::rgba8_eq(palette.danger, [0xF0, 0x30, 0x60])
    }

    pub(crate) fn ftx_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
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
                base: super::pair(bg, text),
                weakest: super::pair(panel_low, text_dim),
                weaker: super::pair(color(0x0F, 0x16, 0x21), text_dim),
                weak: super::pair(panel, text_muted),
                neutral: super::pair(color(0x15, 0x1F, 0x2E), text),
                strong: super::pair(panel_high, text),
                stronger: super::pair(panel_active, text),
                strongest: super::pair(color(0x2B, 0x38, 0x49), text),
            },
            primary: Primary {
                base: super::pair(cyan, bg),
                weak: super::pair(cyan_dark, cyan_bright),
                strong: super::pair(cyan_bright, bg),
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
                base: super::pair(amber, bg),
                weak: super::pair(amber_dark, amber),
                strong: super::pair(color(0xFF, 0xC1, 0x6B), bg),
            },
            danger: Danger {
                base: super::pair(red, text),
                weak: super::pair(red_dark, red),
                strong: super::pair(color(0xFF, 0x55, 0x7C), text),
            },
            is_dark: true,
        }
    }
}
