use crate::trade_feed_state::trade_shading;
use iced::{Color, Theme};

pub(super) struct TraderColors([Color; 8]);

impl TraderColors {
    pub(super) fn new(theme: &Theme) -> Self {
        let palette = theme.palette();
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
        // One color per trader on both buy and sell rows, including maximum tint.
        let backgrounds = [
            palette.background,
            mix(palette.background, palette.success, trade_shading(1.0, 1.0)),
            mix(palette.background, palette.danger, trade_shading(1.0, 1.0)),
        ];
        let text = if backgrounds
            .iter()
            .all(|bg| contrast(palette.text, *bg) >= 4.5)
        {
            palette.text
        } else {
            crate::helpers::text_color_for_bg(palette.background)
        };
        Self(colors.map(|color| {
            (0..=20)
                .map(|step| mix(color, text, step as f32 / 20.0))
                .find(|candidate| {
                    backgrounds
                        .iter()
                        .all(|bg| contrast(*candidate, *bg) >= 4.5)
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

fn mix(a: Color, b: Color, amount: f32) -> Color {
    Color::from_rgb(
        a.r + (b.r - a.r) * amount,
        a.g + (b.g - a.g) * amount,
        a.b + (b.b - a.b) * amount,
    )
}

fn contrast(a: Color, b: Color) -> f32 {
    let luminance = |color: Color| {
        let [r, g, b, _] = color.into_linear();
        r * 0.2126 + g * 0.7152 + b * 0.0722
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
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
                for tint in [theme.palette().success, theme.palette().danger] {
                    for step in 0..=10 {
                        let bg = mix(theme.palette().background, tint, 0.03 * step as f32);
                        assert!(
                            contrast(color, bg) >= 4.5,
                            "unreadable trader color in {theme:?}"
                        );
                    }
                }
            }
        }
    }
}
