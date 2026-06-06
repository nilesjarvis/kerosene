use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_hyperliquid_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x0F, 0x1A, 0x1E])
            && super::rgba8_eq(palette.text, [0xF6, 0xFE, 0xFD])
            && super::rgba8_eq(palette.primary, [0x50, 0xD2, 0xC1])
            && super::rgba8_eq(palette.success, [0x50, 0xD2, 0xC1])
            && super::rgba8_eq(palette.warning, [0xFF, 0xB6, 0x48])
            && super::rgba8_eq(palette.danger, [0xED, 0x70, 0x88])
    }

    pub(crate) fn hyperliquid_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let bg = color(0x0F, 0x1A, 0x1E);
        let text_white = color(0xF6, 0xFE, 0xFD);
        let text_gray = color(0xD2, 0xDA, 0xD7);
        let text_title = color(0x94, 0x9E, 0x9C);
        let text_placeholder = color(0x87, 0x8C, 0x8F);
        let text_black = color(0x04, 0x06, 0x0C);
        let green_300 = color(0x50, 0xD2, 0xC1);
        let green_200 = color(0x97, 0xFC, 0xE4);
        let positive_hover = color(0x2F, 0xB6, 0x8D);
        let negative = color(0xED, 0x70, 0x88);
        let negative_hover = color(0xFD, 0x80, 0x98);
        let yellow = color(0xFF, 0xB6, 0x48);

        Extended {
            background: Background {
                base: super::pair(bg, text_white),
                weakest: super::pair(color(0x0F, 0x1A, 0x1F), text_placeholder),
                weaker: super::pair(color(0x13, 0x15, 0x17), text_placeholder),
                weak: super::pair(color(0x1B, 0x24, 0x29), text_title),
                neutral: super::pair(color(0x22, 0x24, 0x28), text_gray),
                strong: super::pair(color(0x27, 0x30, 0x35), text_white),
                stronger: super::pair(color(0x30, 0x30, 0x30), text_white),
                strongest: super::pair(color(0x3B, 0x42, 0x41), text_white),
            },
            primary: Primary {
                base: super::pair(green_300, text_black),
                weak: super::pair(color(0x07, 0x27, 0x23), green_300),
                strong: super::pair(green_200, text_black),
            },
            secondary: Secondary {
                base: super::pair(text_gray, bg),
                weak: super::pair(text_title, bg),
                strong: super::pair(text_white, bg),
            },
            success: Success {
                base: super::pair(green_300, text_black),
                weak: super::pair(color(0x0E, 0x33, 0x33), green_300),
                strong: super::pair(positive_hover, text_black),
            },
            warning: Warning {
                base: super::pair(yellow, text_black),
                weak: super::pair(color(0x34, 0x2A, 0x1A), yellow),
                strong: super::pair(color(0xFE, 0xF9, 0xA0), text_black),
            },
            danger: Danger {
                base: super::pair(negative, text_black),
                weak: super::pair(color(0x34, 0x24, 0x2E), negative),
                strong: super::pair(negative_hover, text_black),
            },
            is_dark: true,
        }
    }
}
