use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_bybit_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x10, 0x10, 0x14])
            && super::rgba8_eq(palette.text, [0xF5, 0xF5, 0xF5])
            && super::rgba8_eq(palette.primary, [0xF4, 0xB4, 0x44])
            && super::rgba8_eq(palette.success, [0x55, 0xAF, 0x72])
            && super::rgba8_eq(palette.warning, [0xE8, 0xA8, 0x38])
            && super::rgba8_eq(palette.danger, [0xDC, 0x53, 0x51])
    }

    pub(crate) fn bybit_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let bg = color(0x10, 0x10, 0x14);
        let panel_low = color(0x08, 0x08, 0x0C);
        let panel = color(0x17, 0x18, 0x1D);
        let panel_high = color(0x20, 0x21, 0x24);
        let panel_active = color(0x41, 0x43, 0x47);
        let text = color(0xF5, 0xF5, 0xF5);
        let text_muted = color(0xA8, 0xB0, 0xB0);
        let text_dim = color(0x70, 0x75, 0x7A);
        let amber = color(0xF4, 0xB4, 0x44);
        let amber_soft = color(0xFF, 0xCC, 0x5C);
        let amber_dark = color(0x36, 0x27, 0x12);
        let green = color(0x55, 0xAF, 0x72);
        let green_bright = color(0x6D, 0xE5, 0x76);
        let green_dark = color(0x13, 0x2D, 0x20);
        let red = color(0xDC, 0x53, 0x51);
        let red_dark = color(0x35, 0x1A, 0x1D);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(panel_low, text_dim),
                weaker: super::pair(color(0x0B, 0x0C, 0x10), text_dim),
                weak: super::pair(panel, text_muted),
                neutral: super::pair(color(0x1B, 0x1C, 0x21), text),
                strong: super::pair(panel_high, text),
                stronger: super::pair(panel_active, text),
                strongest: super::pair(color(0x4E, 0x51, 0x56), text),
            },
            primary: Primary {
                base: super::pair(amber, bg),
                weak: super::pair(amber_dark, amber),
                strong: super::pair(amber_soft, bg),
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
                base: super::pair(color(0xE8, 0xA8, 0x38), bg),
                weak: super::pair(amber_dark, amber),
                strong: super::pair(amber_soft, bg),
            },
            danger: Danger {
                base: super::pair(red, text),
                weak: super::pair(red_dark, red),
                strong: super::pair(color(0xFF, 0x5A, 0x64), text),
            },
            is_dark: true,
        }
    }
}
