use crate::app_state::TradingTerminal;

use iced::{Color, Theme};

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

use self::color_parse::parse_hex_color;

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

        Theme::Custom(std::sync::Arc::new(iced::theme::Custom::with_fn(
            name,
            palette,
            move |p| {
                use iced::theme::palette::{
                    Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning,
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
                        base: Pair { color: bg, text },
                        weak: Pair {
                            color: mix(bg, text, 0.04),
                            text,
                        },
                        strong: Pair {
                            color: mix(bg, text, 0.08),
                            text,
                        },
                        weaker: Pair {
                            color: mix(bg, text, 0.02),
                            text,
                        },
                        weakest: Pair {
                            color: mix(bg, text, 0.01),
                            text,
                        },
                        neutral: Pair {
                            color: mix(bg, text, 0.06),
                            text,
                        },
                        stronger: Pair {
                            color: mix(bg, text, 0.12),
                            text,
                        },
                        strongest: Pair {
                            color: mix(bg, text, 0.16),
                            text,
                        },
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
        self.get_theme_by_name(&self.active_theme)
    }
}
