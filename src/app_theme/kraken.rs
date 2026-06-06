use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_kraken_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x0B, 0x07, 0x11])
            && super::rgba8_eq(palette.text, [0xE8, 0xE1, 0xF2])
            && super::rgba8_eq(palette.primary, [0x71, 0x32, 0xF5])
            && super::rgba8_eq(palette.success, [0x2B, 0xB6, 0x7B])
            && super::rgba8_eq(palette.warning, [0xED, 0x9B, 0x35])
            && super::rgba8_eq(palette.danger, [0xB2, 0x42, 0x5F])
    }

    pub(crate) fn kraken_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        let bg = color(0x0B, 0x07, 0x11);
        let panel = color(0x17, 0x13, 0x1D);
        let panel_low = color(0x14, 0x0F, 0x1A);
        let panel_high = color(0x21, 0x1D, 0x28);
        let panel_active = color(0x30, 0x2D, 0x3C);
        let text = color(0xE8, 0xE1, 0xF2);
        let text_muted = color(0x9A, 0x93, 0xA7);
        let text_dim = color(0x6E, 0x68, 0x7B);
        let purple = color(0x71, 0x32, 0xF5);
        let purple_soft = color(0x9B, 0x61, 0xBC);
        let purple_dark = color(0x1D, 0x14, 0x3A);
        let green = color(0x2B, 0xB6, 0x7B);
        let green_bright = color(0x43, 0xB7, 0x88);
        let green_dark = color(0x12, 0x3B, 0x32);
        let amber = color(0xED, 0x9B, 0x35);
        let amber_dark = color(0x35, 0x25, 0x18);
        let rose = color(0xB2, 0x42, 0x5F);
        let rose_bright = color(0xE3, 0x4A, 0x6F);
        let rose_dark = color(0x3B, 0x16, 0x26);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(bg, text_dim),
                weaker: super::pair(panel_low, text_dim),
                weak: super::pair(panel, text_muted),
                neutral: super::pair(color(0x1A, 0x16, 0x20), text),
                strong: super::pair(panel_high, text),
                stronger: super::pair(panel_active, text),
                strongest: super::pair(color(0x3B, 0x36, 0x49), text),
            },
            primary: Primary {
                base: super::pair(purple, text),
                weak: super::pair(purple_dark, purple_soft),
                strong: super::pair(purple_soft, text),
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
                strong: super::pair(color(0xF9, 0xCF, 0x85), bg),
            },
            danger: Danger {
                base: super::pair(rose, text),
                weak: super::pair(rose_dark, rose_bright),
                strong: super::pair(rose_bright, text),
            },
            is_dark: true,
        }
    }
}
