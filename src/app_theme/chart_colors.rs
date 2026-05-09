use super::color_parse::parse_hex_color;
use crate::app_state::TradingTerminal;
use iced::{Color, Theme};

impl TradingTerminal {
    pub(crate) fn chart_theme_colors_for(
        &self,
        theme_name: &str,
    ) -> (Option<Color>, Option<Color>) {
        let Some(name) = theme_name.strip_prefix("Custom: ") else {
            return (None, None);
        };
        let Some(theme) = self.custom_themes.iter().find(|t| t.name == name) else {
            return (None, None);
        };

        (
            theme.chart_bull.as_deref().and_then(parse_hex_color),
            theme.chart_bear.as_deref().and_then(parse_hex_color),
        )
    }

    pub(crate) fn active_chart_theme_colors(&self) -> (Option<Color>, Option<Color>) {
        self.chart_theme_colors_for(&self.active_theme)
    }

    pub(crate) fn direction_colors(&self, theme: &Theme) -> (Color, Color) {
        let (up, down) = self.active_chart_theme_colors();
        (
            up.unwrap_or(theme.palette().success),
            down.unwrap_or(theme.palette().danger),
        )
    }

    pub(crate) fn direction_color(&self, theme: &Theme, value: f64) -> Color {
        let (up, down) = self.direction_colors(theme);
        if value >= 0.0 { up } else { down }
    }

    pub(crate) fn apply_chart_theme_colors(&mut self) {
        let (bull, bear) = self.active_chart_theme_colors();
        for instance in self.charts.values_mut() {
            instance.chart.set_chart_colors(bull, bear);
            instance.chart.candle_cache.clear();
        }
    }
}
