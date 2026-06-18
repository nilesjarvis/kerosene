use super::color_parse::parse_hex_color;
use crate::app_state::TradingTerminal;
use iced::{Color, Theme};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ChartThemeOverrides {
    pub(crate) bull: Option<Color>,
    pub(crate) bear: Option<Color>,
    pub(crate) line: Option<Color>,
    pub(crate) line_gradient: Option<Color>,
}

impl TradingTerminal {
    pub(crate) fn chart_theme_overrides_for(&self, theme_name: &str) -> ChartThemeOverrides {
        let Some(name) = theme_name.strip_prefix("Custom: ") else {
            return ChartThemeOverrides::default();
        };
        let Some(theme) = self.custom_themes.iter().find(|t| t.name == name) else {
            return ChartThemeOverrides::default();
        };

        ChartThemeOverrides {
            bull: theme.chart_bull.as_deref().and_then(parse_hex_color),
            bear: theme.chart_bear.as_deref().and_then(parse_hex_color),
            line: theme.chart_line.as_deref().and_then(parse_hex_color),
            line_gradient: theme
                .chart_line_gradient
                .as_deref()
                .and_then(parse_hex_color),
        }
    }

    pub(crate) fn active_chart_theme_overrides(&self) -> ChartThemeOverrides {
        self.chart_theme_overrides_for(&self.active_theme)
    }

    pub(crate) fn direction_colors(&self, theme: &Theme) -> (Color, Color) {
        let overrides = self.active_chart_theme_overrides();
        (
            overrides.bull.unwrap_or(theme.palette().success),
            overrides.bear.unwrap_or(theme.palette().danger),
        )
    }

    pub(crate) fn direction_color(&self, theme: &Theme, value: f64) -> Color {
        let (up, down) = self.direction_colors(theme);
        if value >= 0.0 { up } else { down }
    }

    pub(crate) fn apply_chart_theme_colors(&mut self) {
        let theme = self.theme();
        let overrides = self.active_chart_theme_overrides();
        for instance in self.charts.values_mut() {
            instance.chart.set_chart_theme_overrides(overrides);
            instance.chart.candle_cache.clear();
        }
        for instance in self.spaghetti_charts.values_mut() {
            instance.canvas.apply_style_colors(&theme);
        }
    }
}
