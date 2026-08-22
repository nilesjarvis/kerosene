use crate::app_state::TradingTerminal;

use iced::{Color, Theme, theme};

mod bloomberg;
mod bybit;
mod chart_colors;
mod coinbase_dark;
mod coinbase_light;
mod color_parse;
mod ftx;
mod hyperliquid;
mod ibkr_dark;
mod kraken;
mod kwenta;
mod ubuntu;

pub(crate) use chart_colors::ChartThemeOverrides;

use self::color_parse::parse_hex_color;

pub(super) fn rgba8_eq(color: Color, rgb: [u8; 3]) -> bool {
    color.into_rgba8() == [rgb[0], rgb[1], rgb[2], 255]
}

pub(super) fn pair(color: Color, text: Color) -> iced::theme::palette::Pair {
    iced::theme::palette::Pair { color, text }
}

// ---------------------------------------------------------------------------
// Theme construction
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub fn get_theme_by_name(&self, theme_name: &str) -> Theme {
        let base_theme = match theme_name {
            "Dark" => Theme::Dark,
            "Light" => Theme::Light,
            "Dracula" => Theme::Dracula,
            "Nord" => Theme::Nord,
            "Solarized Dark" => Theme::SolarizedDark,
            "Solarized Light" => Theme::SolarizedLight,
            "Gruvbox Dark" => Theme::GruvboxDark,
            "Gruvbox Light" => Theme::GruvboxLight,
            "Catppuccin Macchiato" => Theme::CatppuccinMacchiato,
            "Catppuccin Mocha" => Theme::CatppuccinMocha,
            "Tokyo Night" => Theme::TokyoNight,
            "Tokyo Night Storm" => Theme::TokyoNightStorm,
            "Tokyo Night Light" => Theme::TokyoNightLight,
            "Kanagawa Wave" => Theme::KanagawaWave,
            "Kanagawa Dragon" => Theme::KanagawaDragon,
            "Kanagawa Lotus" => Theme::KanagawaLotus,
            "Moonfly" => Theme::Moonfly,
            "Nightfly" => Theme::Nightfly,
            "Oxocarbon" => Theme::Oxocarbon,
            "Ferra" => Theme::Ferra,
            custom if custom.starts_with("Custom: ") => {
                let name = custom.trim_start_matches("Custom: ");
                if let Some(ct) = self.custom_themes.iter().find(|t| t.name == name) {
                    let parse_color = |hex: &str| parse_hex_color(hex).unwrap_or(Color::BLACK);

                    use iced::theme::Palette;
                    let bg = parse_color(&ct.background);
                    let text = parse_color(&ct.text);
                    let p = Palette {
                        background: bg,
                        text,
                        primary: parse_color(&ct.primary),
                        success: parse_color(&ct.success),
                        danger: parse_color(&ct.danger),
                        warning: parse_color(&ct.warning),
                    };

                    Theme::Custom(std::sync::Arc::new(iced::theme::Custom::new(
                        name.to_string(),
                        p,
                    )))
                } else {
                    Theme::Dark
                }
            }
            _ => Theme::Dark,
        };

        let palette = base_theme.palette();
        let bg = palette.background;
        let text = palette.text;
        let name = theme_name.to_string();
        let use_hyperliquid_source_palette = theme_name == "Custom: Hyperliquid"
            && Self::palette_matches_hyperliquid_source(palette);
        let use_bloomberg_source_palette =
            theme_name == "Custom: Bloomberg" && Self::palette_matches_bloomberg_source(palette);
        let use_kraken_source_palette =
            theme_name == "Custom: Kraken" && Self::palette_matches_kraken_source(palette);
        let use_ftx_source_palette =
            theme_name == "Custom: FTX" && Self::palette_matches_ftx_source(palette);
        let use_ibkr_dark_source_palette =
            theme_name == "Custom: IBKR Dark" && Self::palette_matches_ibkr_dark_source(palette);
        let use_bybit_source_palette =
            theme_name == "Custom: bybit" && Self::palette_matches_bybit_source(palette);
        let use_coinbase_dark_source_palette = theme_name == "Custom: coinbase-dark"
            && Self::palette_matches_coinbase_dark_source(palette);
        let use_coinbase_light_source_palette = theme_name == "Custom: coinbase-light"
            && Self::palette_matches_coinbase_light_source(palette);
        let use_kwenta_source_palette =
            theme_name == "Custom: kwenta" && Self::palette_matches_kwenta_source(palette);
        let use_ubuntu_source_palette =
            theme_name == "Custom: ubuntu" && Self::palette_matches_ubuntu_source(palette);

        Theme::Custom(std::sync::Arc::new(iced::theme::Custom::with_fn(
            name,
            palette,
            move |p| {
                use iced::theme::palette::{
                    Background, Danger, Extended, Primary, Secondary, Success, Warning,
                };

                if use_hyperliquid_source_palette
                    && TradingTerminal::palette_matches_hyperliquid_source(p)
                {
                    return TradingTerminal::hyperliquid_source_extended_palette();
                }
                if use_bloomberg_source_palette
                    && TradingTerminal::palette_matches_bloomberg_source(p)
                {
                    return TradingTerminal::bloomberg_source_extended_palette();
                }
                if use_kraken_source_palette && TradingTerminal::palette_matches_kraken_source(p) {
                    return TradingTerminal::kraken_source_extended_palette();
                }
                if use_ftx_source_palette && TradingTerminal::palette_matches_ftx_source(p) {
                    return TradingTerminal::ftx_source_extended_palette();
                }
                if use_ibkr_dark_source_palette
                    && TradingTerminal::palette_matches_ibkr_dark_source(p)
                {
                    return TradingTerminal::ibkr_dark_source_extended_palette();
                }
                if use_bybit_source_palette && TradingTerminal::palette_matches_bybit_source(p) {
                    return TradingTerminal::bybit_source_extended_palette();
                }
                if use_coinbase_dark_source_palette
                    && TradingTerminal::palette_matches_coinbase_dark_source(p)
                {
                    return TradingTerminal::coinbase_dark_source_extended_palette();
                }
                if use_coinbase_light_source_palette
                    && TradingTerminal::palette_matches_coinbase_light_source(p)
                {
                    return TradingTerminal::coinbase_light_source_extended_palette();
                }
                if use_kwenta_source_palette && TradingTerminal::palette_matches_kwenta_source(p) {
                    return TradingTerminal::kwenta_source_extended_palette();
                }
                if use_ubuntu_source_palette && TradingTerminal::palette_matches_ubuntu_source(p) {
                    return TradingTerminal::ubuntu_source_extended_palette();
                }

                fn mix(a: Color, b: Color, factor: f32) -> Color {
                    Color::from_rgba(
                        a.r + (b.r - a.r) * factor,
                        a.g + (b.g - a.g) * factor,
                        a.b + (b.b - a.b) * factor,
                        a.a + (b.a - a.a) * factor,
                    )
                }

                Extended {
                    background: Background {
                        base: pair(bg, text),
                        weak: pair(mix(bg, text, 0.04), text),
                        strong: pair(mix(bg, text, 0.08), text),
                        weaker: pair(mix(bg, text, 0.02), text),
                        weakest: pair(mix(bg, text, 0.01), text),
                        neutral: pair(mix(bg, text, 0.06), text),
                        stronger: pair(mix(bg, text, 0.12), text),
                        strongest: pair(mix(bg, text, 0.16), text),
                    },
                    primary: Primary::generate(p.primary, bg, text),
                    secondary: Secondary::generate(p.primary, text),
                    success: Success::generate(p.success, bg, text),
                    danger: Danger::generate(p.danger, bg, text),
                    warning: Warning::generate(p.warning, bg, text),
                    is_dark: {
                        let bg_lin = bg.into_linear();
                        let lum = bg_lin[0] * 0.2126 + bg_lin[1] * 0.7152 + bg_lin[2] * 0.0722;
                        lum < 0.5
                    },
                }
            },
        )))
    }

    pub fn theme(&self) -> Theme {
        let theme = self.get_theme_by_name(&self.active_theme);
        if self.window_transparency_enabled {
            with_background_opacity(theme, self.window_background_opacity)
        } else {
            theme
        }
    }

    pub(crate) fn application_style(state: &Self, theme: &Theme) -> theme::Style {
        theme::Style {
            background_color: if state.window_transparency_enabled {
                Color::TRANSPARENT
            } else {
                theme.palette().background
            },
            text_color: theme.palette().text,
        }
    }
}

fn with_background_opacity(theme: Theme, opacity: f32) -> Theme {
    let opacity = crate::config::normalize_window_background_opacity(opacity);
    let mut palette = theme.palette();
    let mut extended = *theme.extended_palette();
    let name = format!("{theme} / transparent {opacity:.3}");

    palette.background.a *= opacity;
    for pair in [
        &mut extended.background.base,
        &mut extended.background.weakest,
        &mut extended.background.weaker,
        &mut extended.background.weak,
        &mut extended.background.neutral,
        &mut extended.background.strong,
        &mut extended.background.stronger,
        &mut extended.background.strongest,
    ] {
        pair.color.a *= opacity;
    }

    Theme::custom_with_fn(name, palette, move |_| extended)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_theme_only_reduces_background_surface_alpha() {
        let source = Theme::Dark;
        let source_extended = *source.extended_palette();
        let transparent = with_background_opacity(source.clone(), 0.6);
        let extended = transparent.extended_palette();

        assert_eq!(transparent.palette().background.a, 0.6);
        assert_eq!(extended.background.base.color.a, 0.6);
        assert_eq!(extended.background.strong.color.a, 0.6);
        assert_eq!(transparent.palette().text, source.palette().text);
        assert_eq!(extended.primary.base, source_extended.primary.base);
        assert_eq!(extended.success.base, source_extended.success.base);
    }

    #[test]
    fn transparent_application_clear_does_not_add_an_opaque_layer() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.window_transparency_enabled = true;
        terminal.window_background_opacity = 0.7;
        let theme = terminal.theme();

        let style = TradingTerminal::application_style(&terminal, &theme);

        assert_eq!(style.background_color, Color::TRANSPARENT);
        assert_eq!(style.text_color, theme.palette().text);
    }

    #[test]
    fn boot_normalizes_invalid_window_background_opacity() {
        let config = crate::config::KeroseneConfig {
            window_transparency_enabled: true,
            window_background_blur_enabled: true,
            window_background_opacity: f32::INFINITY,
            ..crate::config::KeroseneConfig::default()
        };

        let (terminal, _) = TradingTerminal::boot_from_config(config);

        assert!(terminal.window_transparency_enabled);
        assert!(terminal.window_background_blur_enabled);
        assert_eq!(
            terminal.window_background_opacity,
            crate::config::DEFAULT_WINDOW_BACKGROUND_OPACITY
        );
    }
}
