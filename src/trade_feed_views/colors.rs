use crate::helpers::design::{UiColors, contrast_ratio as contrast, mix_color as mix};
use iced::{Color, Theme};

pub(super) struct TraderColors([Color; 8]);

impl TraderColors {
    pub(super) fn new(theme: &Theme) -> Self {
        let palette = theme.palette();
        let ui = UiColors::new(theme);
        let colors = [
            palette.primary,
            mix(palette.primary, palette.success, 0.5),
            palette.success,
            mix(palette.success, palette.warning, 0.5),
            palette.warning,
            mix(palette.warning, palette.danger, 0.5),
            palette.danger,
            mix(palette.danger, palette.primary, 0.5),
        ];
        // Resolve one stable foreground per trader against the entire intensity
        // range on both sides. The minimum luminance can lie between endpoints.
        let backgrounds: Vec<_> = [palette.success, palette.danger]
            .into_iter()
            .flat_map(|tint| (0..=30).map(move |step| row_background(ui, tint, step as f32 * 0.01)))
            .collect();
        let by_luminance = |a: &&Color, b: &&Color| {
            contrast(Color::BLACK, **a).total_cmp(&contrast(Color::BLACK, **b))
        };
        let backgrounds = [
            backgrounds
                .iter()
                .min_by(by_luminance)
                .copied()
                .unwrap_or(ui.panel),
            backgrounds
                .iter()
                .max_by(by_luminance)
                .copied()
                .unwrap_or(ui.panel),
        ];
        let minimum = |color| {
            backgrounds
                .iter()
                .map(|bg| contrast(color, *bg))
                .fold(f32::INFINITY, f32::min)
        };
        let text = if minimum(Color::BLACK) > minimum(Color::WHITE) {
            Color::BLACK
        } else {
            Color::WHITE
        };
        Self(colors.map(|color| {
            (0..=100)
                .map(|step| mix(color, text, step as f32 / 100.0))
                .find(|candidate| {
                    backgrounds
                        .iter()
                        .all(|bg| contrast(*candidate, *bg) >= 4.55)
                })
                .unwrap_or(text)
        }))
    }

    pub(super) fn for_address(&self, address: &str) -> Color {
        // Fixed FNV-1a, rather than a randomized or row-order-dependent assignment.
        // Hash the whole normalized address, never its label or truncated display.
        let hash = address
            .trim()
            .bytes()
            .fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(byte.to_ascii_lowercase())).wrapping_mul(0x100000001b3)
            });
        self.0[(hash % self.0.len() as u64) as usize]
    }
}

pub(super) fn row_background(colors: UiColors, tint: Color, shading: f32) -> Color {
    // Notional still determines intensity, bounded to a quiet 12% surface tint.
    // Reduce it further if necessary to keep the primary foreground readable.
    (0..=12)
        .rev()
        .map(|step| mix(colors.panel, tint, shading * 0.4 * step as f32 / 12.0))
        .find(|background| contrast(colors.text, *background) >= 4.5)
        .unwrap_or(colors.panel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_feed_trader_colors_are_stable_case_insensitive_and_use_the_full_address() {
        let colors = TraderColors::new(&Theme::Dark);
        let address = "0xabcdef0000000000000000000000000000001234";
        let original = colors.for_address(address);
        assert_eq!(colors.for_address(&address.to_uppercase()), original);
        // Both addresses have the same shortened label, but different interiors.
        assert_ne!(
            colors.for_address("0xabcdef0000000000000100000000000000001234"),
            original
        );
        for other in 0..64 {
            let _ = colors.for_address(&format!("0x{other:040x}"));
        }
        assert_eq!(colors.for_address(address), original);
        assert_eq!(
            TraderColors::new(&Theme::Dark).for_address(address),
            original
        );
    }

    #[test]
    fn trade_feed_trader_colors_are_distinct_and_readable_on_light_and_dark_tints() {
        for theme in [
            Theme::Dark,
            Theme::Light,
            Theme::Dracula,
            Theme::SolarizedLight,
        ] {
            let colors = TraderColors::new(&theme);
            let distinct: std::collections::HashSet<_> =
                colors.0.iter().map(|color| color.into_rgba8()).collect();
            assert_eq!(
                distinct.len(),
                colors.0.len(),
                "collapsed colors in {theme:?}"
            );
            for color in colors.0 {
                for buy in [true, false] {
                    for step in 0..=10 {
                        let bg = row_background(
                            UiColors::new(&theme),
                            if buy {
                                theme.palette().success
                            } else {
                                theme.palette().danger
                            },
                            0.03 * step as f32,
                        );
                        assert!(
                            contrast(color, bg) >= 4.5,
                            "unreadable trader color in {theme:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn trader_colors_cover_actual_panel_surfaces_in_every_theme() {
        let (terminal, _) = crate::app_state::TradingTerminal::boot_from_config(
            crate::config::KeroseneConfig::default(),
        );
        let names: Vec<_> = Theme::ALL
            .iter()
            .map(ToString::to_string)
            .chain(
                terminal
                    .custom_themes
                    .iter()
                    .map(|theme| format!("Custom: {}", theme.name)),
            )
            .collect();
        for name in names {
            let theme = terminal.get_theme_by_name(&name);
            let colors = TraderColors::new(&theme);
            for buy in [true, false] {
                for step in 0..=10 {
                    let background = row_background(
                        UiColors::new(&theme),
                        if buy {
                            theme.palette().success
                        } else {
                            theme.palette().danger
                        },
                        step as f32 * 0.03,
                    );
                    for color in colors.0 {
                        assert!(
                            contrast(color, background) >= 4.5,
                            "{name}, buy={buy}, shade={step}"
                        );
                    }
                }
            }
        }
    }
}
