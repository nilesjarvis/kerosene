use crate::app_state::TradingTerminal;
use iced::Color;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn palette_matches_papertrade_uwu_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0x0D, 0x12, 0x29])
            && super::rgba8_eq(palette.text, [0xF4, 0xEA, 0xF2])
            && super::rgba8_eq(palette.primary, [0xE4, 0xB4, 0xCE])
            && super::rgba8_eq(palette.success, [0x91, 0xD8, 0xBD])
            && super::rgba8_eq(palette.warning, [0xF0, 0xD0, 0x9A])
            && super::rgba8_eq(palette.danger, [0xF0, 0xA0, 0xB2])
    }

    pub(crate) fn papertrade_uwu_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        // UwU's dark CSS tokens, captured in assets/screenshots/papertrade/uwu-styles.json.
        let bg = color(0x0D, 0x12, 0x29);
        let panel = color(0x17, 0x1D, 0x3D);
        let text = color(0xF4, 0xEA, 0xF2);
        let muted = color(0xC7, 0xC4, 0xD8);
        let faint = color(0xA4, 0xAC, 0xCB);
        let on_accent = color(0x0E, 0x13, 0x29);
        let pink = color(0xE4, 0xB4, 0xCE);
        let lavender = color(0xC5, 0xC9, 0xF2);
        let green = color(0x91, 0xD8, 0xBD);
        let red = color(0xF0, 0xA0, 0xB2);
        // Use the source's dark chart warning, rather than its inherited light-mode ochre.
        let warning = color(0xF0, 0xD0, 0x9A);
        // Iced's palette stores solid colors: use gradient endpoints for base/hover
        // controls and translucent accent tints for weak states.
        let tinted = |accent: Color| {
            Color::from_rgb(
                panel.r * 0.85 + accent.r * 0.15,
                panel.g * 0.85 + accent.g * 0.15,
                panel.b * 0.85 + accent.b * 0.15,
            )
        };

        Extended {
            background: Background {
                base: super::pair(bg, text),
                weakest: super::pair(color(0x08, 0x0C, 0x1D), faint),
                weaker: super::pair(color(0x0D, 0x14, 0x30), faint),
                weak: super::pair(color(0x11, 0x18, 0x32), muted),
                neutral: super::pair(color(0x15, 0x1E, 0x3D), text),
                // Pane bodies and flat chart backgrounds use `strong`.
                strong: super::pair(panel, text),
                stronger: super::pair(color(0x27, 0x2F, 0x5A), text),
                strongest: super::pair(color(0x46, 0x53, 0x82), text),
            },
            primary: Primary {
                base: super::pair(pink, on_accent),
                weak: super::pair(tinted(pink), pink),
                strong: super::pair(lavender, on_accent),
            },
            secondary: Secondary {
                base: super::pair(color(0xB8, 0xC5, 0xFF), on_accent),
                weak: super::pair(color(0x46, 0x53, 0x82), text),
                strong: super::pair(text, on_accent),
            },
            success: Success {
                base: super::pair(green, on_accent),
                weak: super::pair(tinted(green), green),
                strong: super::pair(color(0xA8, 0xE7, 0xD1), on_accent),
            },
            warning: Warning {
                base: super::pair(warning, on_accent),
                weak: super::pair(tinted(warning), warning),
                strong: super::pair(warning, on_accent),
            },
            danger: Danger {
                base: super::pair(red, on_accent),
                weak: super::pair(tinted(red), red),
                strong: super::pair(color(0xFF, 0x91, 0xA7), on_accent),
            },
            is_dark: true,
        }
    }
}
