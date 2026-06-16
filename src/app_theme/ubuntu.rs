use crate::app_state::TradingTerminal;
use iced::Color;

impl TradingTerminal {
    pub(crate) fn palette_matches_ubuntu_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x1B, 0x0E, 0x18])
            && super::rgba8_eq(palette.text, [0xF3, 0xEA, 0xEF])
            && super::rgba8_eq(palette.primary, [0xF6, 0x6D, 0x2C])
            && super::rgba8_eq(palette.success, [0x33, 0xD1, 0x7A])
            && super::rgba8_eq(palette.warning, [0xFF, 0xD2, 0x4A])
            && super::rgba8_eq(palette.danger, [0xF5, 0x46, 0x5F])
    }

    pub(crate) fn ubuntu_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        // Aubergine ground: the whole ladder and both muted greys inherit the
        // Canonical eggplant hue, so the warm orange/gold/rose accents pop against
        // unified purple chrome instead of fighting a clashing neutral grey. The
        // 8 surfaces are monotonic in luminance (well < base < ... < active).
        let bg = color(0x1B, 0x0E, 0x18);
        let well = color(0x14, 0x0A, 0x11);
        let panel_low = color(0x24, 0x16, 0x20);
        let panel = color(0x2C, 0x1B, 0x27);
        let input = color(0x35, 0x23, 0x32);
        let card = color(0x41, 0x2B, 0x3D);
        let hover = color(0x50, 0x39, 0x4C);
        // Active/selected stops at #62475D rather than the literal Canonical
        // Aubergine (#772953) so muted text on the most-touched row state still
        // clears AA; the brand feel survives in the hue, not the exact hex.
        let active = color(0x62, 0x47, 0x5D);
        let text = color(0xF3, 0xEA, 0xEF);
        let text_muted = color(0xC4, 0xB3, 0xBF);
        let text_dim = color(0x91, 0x7E, 0x8B);
        // Ubuntu Orange hero, lifted from brand #E95420 (only ~1.6:1 here) to
        // clear AA on the dark base while staying recognizably Ubuntu orange.
        let orange = color(0xF6, 0x6D, 0x2C);
        let orange_bright = color(0xFF, 0x8A, 0x4D);
        let orange_dark = color(0x3A, 0x1B, 0x0C);
        let green = color(0x33, 0xD1, 0x7A);
        let green_bright = color(0x5F, 0xE3, 0x9B);
        let green_dark = color(0x0C, 0x33, 0x22);
        // Gold caution, lifted to be the unambiguously brightest warm accent so it
        // never collides with the orange hero (both warm, so lightness separates).
        let gold = color(0xFF, 0xD2, 0x4A);
        let gold_bright = color(0xFF, 0xE0, 0x7A);
        let gold_dark = color(0x3A, 0x2E, 0x08);
        // Rose-red danger deepened so loss reads as both a different hue AND
        // visibly darker than the orange button, even under deuteranopia.
        let red = color(0xF5, 0x46, 0x5F);
        let red_bright = color(0xFF, 0x73, 0x88);
        let red_dark = color(0x3A, 0x0F, 0x18);

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(well, text_dim),
                weaker: super::pair(panel_low, text_dim),
                weak: super::pair(panel, text_muted),
                neutral: super::pair(input, text),
                strong: super::pair(card, text),
                stronger: super::pair(hover, text),
                strongest: super::pair(active, text),
            },
            primary: Primary {
                base: super::pair(orange, bg),
                weak: super::pair(orange_dark, orange_bright),
                strong: super::pair(orange_bright, bg),
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
                base: super::pair(gold, bg),
                weak: super::pair(gold_dark, gold_bright),
                strong: super::pair(gold_bright, bg),
            },
            danger: Danger {
                base: super::pair(red, bg),
                weak: super::pair(red_dark, red_bright),
                strong: super::pair(red_bright, bg),
            },
            is_dark: true,
        }
    }
}
