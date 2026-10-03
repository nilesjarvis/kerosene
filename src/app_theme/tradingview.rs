use crate::app_state::TradingTerminal;
use iced::Color;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn palette_matches_tradingview_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x13, 0x17, 0x22])
            && super::rgba8_eq(palette.text, [0xD1, 0xD4, 0xDC])
            && super::rgba8_eq(palette.primary, [0x29, 0x62, 0xFF])
            && super::rgba8_eq(palette.success, [0x26, 0xA6, 0x9A])
            && super::rgba8_eq(palette.warning, [0xFF, 0x98, 0x00])
            && super::rgba8_eq(palette.danger, [0xEF, 0x53, 0x50])
    }

    pub(crate) fn tradingview_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        // Classic dark TradingView chart reference linked in the theme documentation.
        let bg = color(0x13, 0x17, 0x22);
        let text = color(0xD1, 0xD4, 0xDC);
        let text_muted = color(0xB2, 0xB5, 0xBE);
        let text_dim = color(0x78, 0x7B, 0x86);
        let white = Color::WHITE;
        let blue = color(0x29, 0x62, 0xFF);
        let teal = color(0x26, 0xA6, 0x9A);
        let orange = color(0xFF, 0x98, 0x00);
        let red = color(0xEF, 0x53, 0x50);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(color(0x0C, 0x0E, 0x15), text_dim),
                weaker: super::pair(color(0x10, 0x13, 0x1C), text_dim),
                weak: super::pair(color(0x1E, 0x22, 0x2D), text_muted),
                neutral: super::pair(color(0x2A, 0x2E, 0x39), text),
                // Pane bodies and flat chart backgrounds use `strong`.
                strong: super::pair(bg, text),
                stronger: super::pair(color(0x36, 0x3A, 0x45), text),
                strongest: super::pair(color(0x43, 0x46, 0x51), white),
            },
            primary: Primary {
                base: super::pair(blue, white),
                weak: super::pair(color(0x14, 0x2E, 0x61), color(0x90, 0xBF, 0xF9)),
                strong: super::pair(color(0x1E, 0x53, 0xE5), white),
            },
            secondary: Secondary {
                base: super::pair(text_muted, bg),
                weak: super::pair(text_dim, bg),
                strong: super::pair(text, bg),
            },
            success: Success {
                base: super::pair(teal, bg),
                weak: super::pair(color(0x13, 0x30, 0x32), teal),
                strong: super::pair(color(0x4D, 0xB6, 0xAC), bg),
            },
            warning: Warning {
                base: super::pair(orange, bg),
                weak: super::pair(color(0x33, 0x29, 0x1D), orange),
                strong: super::pair(color(0xFF, 0xB7, 0x4D), bg),
            },
            danger: Danger {
                base: super::pair(red, bg),
                weak: super::pair(color(0x33, 0x20, 0x29), red),
                strong: super::pair(color(0xF2, 0x77, 0x74), bg),
            },
            is_dark: true,
        }
    }
}
