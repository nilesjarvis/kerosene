use crate::app_state::TradingTerminal;
use iced::Color;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn palette_matches_papertrade_source(palette: iced::theme::Palette) -> bool {
        super::rgba8_eq(palette.background, [0xF4, 0xEC, 0xD8])
            && super::rgba8_eq(palette.text, [0x2A, 0x25, 0x20])
            && super::rgba8_eq(palette.primary, [0x7A, 0x62, 0x35])
            && super::rgba8_eq(palette.success, [0x2D, 0x5F, 0x3A])
            && super::rgba8_eq(palette.warning, [0x9C, 0x6A, 0x00])
            && super::rgba8_eq(palette.danger, [0x8B, 0x20, 0x20])
    }

    pub(crate) fn papertrade_source_extended_palette() -> iced::theme::palette::Extended {
        use iced::theme::palette::{
            Background, Danger, Extended, Primary, Secondary, Success, Warning,
        };

        let color = Color::from_rgb8;
        // Papertrade's light CSS tokens, captured in assets/screenshots/papertrade/.
        let cream = color(0xF4, 0xEC, 0xD8);
        let ink = color(0x2A, 0x25, 0x20);
        let muted = color(0x7A, 0x70, 0x60);
        let faint = color(0x5F, 0x56, 0x4B);
        let raised = color(0xFA, 0xF6, 0xED);
        let border = color(0xD4, 0xCB, 0xBA);
        let accent = color(0x7A, 0x62, 0x35);
        let green = color(0x2D, 0x5F, 0x3A);
        let red = color(0x8B, 0x20, 0x20);
        let warning = color(0x9C, 0x6A, 0x00);
        // CSS selected buy/sell controls use a 15% accent over their raised surface.
        let selected = |accent: Color| {
            color(
                ((raised.r * 0.85 + accent.r * 0.15) * 255.0).round() as u8,
                ((raised.g * 0.85 + accent.g * 0.15) * 255.0).round() as u8,
                ((raised.b * 0.85 + accent.b * 0.15) * 255.0).round() as u8,
            )
        };

        Extended {
            background: Background {
                base: super::pair(cream, ink),
                weakest: super::pair(color(0xFB, 0xF8, 0xF1), faint),
                weaker: super::pair(raised, muted),
                weak: super::pair(color(0xED, 0xE5, 0xD0), muted),
                neutral: super::pair(color(0xE8, 0xDE, 0xC8), ink),
                // Pane bodies and flat chart backgrounds use `strong`.
                strong: super::pair(cream, ink),
                stronger: super::pair(border, ink),
                strongest: super::pair(color(0xB8, 0xAD, 0x92), ink),
            },
            primary: Primary {
                base: super::pair(accent, cream),
                weak: super::pair(selected(accent), accent),
                strong: super::pair(ink, cream),
            },
            secondary: Secondary {
                base: super::pair(muted, raised),
                weak: super::pair(border, ink),
                strong: super::pair(ink, raised),
            },
            success: Success {
                base: super::pair(green, cream),
                weak: super::pair(selected(green), green),
                strong: super::pair(color(0x23, 0x4B, 0x2D), cream),
            },
            warning: Warning {
                base: super::pair(warning, raised),
                weak: super::pair(selected(warning), accent),
                strong: super::pair(accent, raised),
            },
            danger: Danger {
                base: super::pair(red, cream),
                weak: super::pair(selected(red), red),
                strong: super::pair(red, raised),
            },
            is_dark: false,
        }
    }
}
