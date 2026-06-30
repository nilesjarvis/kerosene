use super::Session;
use crate::api::{Candle, is_valid_candle};

use iced::widget::canvas;
use iced::{Color, Theme};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Comparison Style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComparisonColorMode {
    #[default]
    Multi,
    Single,
}

impl ComparisonColorMode {
    pub const ALL: [Self; 2] = [Self::Multi, Self::Single];
}

impl std::fmt::Display for ComparisonColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Multi => write!(f, "Multi-color"),
            Self::Single => write!(f, "Single color"),
        }
    }
}

/// Color palette for series lines.
pub fn series_colors(theme: &Theme) -> Vec<Color> {
    vec![
        theme.palette().success,                         // green
        theme.palette().danger,                          // red
        theme.palette().primary,                         // blue
        theme.extended_palette().secondary.base.color,   // yellow
        theme.extended_palette().secondary.strong.color, // purple
        theme.extended_palette().primary.weak.color,     // pink
        theme.extended_palette().primary.strong.color,   // cyan
        theme.extended_palette().danger.weak.color,      // orange
    ]
}

/// A single series in the comparison chart.
pub struct Series {
    pub symbol: String,
    pub display: String,
    pub candles: Vec<Candle>,
    pub color: Color,
    pub loaded: bool,
}

/// The comparison chart canvas state.
pub struct SpaghettiCanvas {
    pub series: Vec<Series>,
    pub cache: canvas::Cache,
    /// Render all comparison series with one theme color or each with its assigned color.
    pub color_mode: ComparisonColorMode,
    /// Show horizontal ticker labels at the latest visible value for each comparison series.
    pub show_labels: bool,
    /// If true and at least two series are loaded, render A/B ratio.
    pub pair_ratio_mode: bool,
    /// In pair ratio mode, render as candlesticks when true.
    pub pair_candle_mode: bool,
    /// Whether chart plot backgrounds use a dotted pattern instead of grid lines.
    pub(crate) dotted_background: bool,
    /// Opacity applied to dotted chart plot backgrounds.
    pub(crate) dotted_background_opacity: f32,
    /// Whether chart plot backgrounds use a theme-aware gradient.
    pub(crate) gradient_background: bool,
    /// Which pair-ratio candle bodies render hollow instead of filled.
    pub(crate) hollow_candle_mode: crate::config::ChartHollowCandleMode,
    /// Reticle style used for the chart crosshair.
    pub(crate) crosshair_style: crate::config::ChartCrosshairStyle,
    /// Whether the chart crosshair draws full-span guide lines.
    pub(crate) crosshair_guides_enabled: bool,
    /// Multiplier applied to local crosshair reticle geometry.
    pub(crate) crosshair_scale: f32,
    /// Monotonic token used to request viewport reset.
    pub reset_epoch: u64,
    /// Session-based normalization start time (ms). If None, uses the
    /// global earliest candle timestamp as a stable base.
    pub base_timestamp: Option<u64>,
    /// Which session is currently selected (for button highlighting).
    pub active_session: Option<Session>,
}

impl SpaghettiCanvas {
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            cache: canvas::Cache::new(),
            color_mode: ComparisonColorMode::default(),
            show_labels: false,
            pair_ratio_mode: false,
            pair_candle_mode: false,
            dotted_background: false,
            dotted_background_opacity: crate::config::default_chart_dotted_background_opacity(),
            gradient_background: false,
            hollow_candle_mode: Default::default(),
            crosshair_style: Default::default(),
            crosshair_guides_enabled: true,
            crosshair_scale: crate::config::default_chart_crosshair_scale(),
            reset_epoch: 0,
            base_timestamp: None,
            active_session: None,
        }
    }

    pub(super) fn loaded_series(&self) -> Vec<&Series> {
        self.series
            .iter()
            .filter(|s| s.loaded && !s.candles.is_empty())
            .collect()
    }

    pub fn effective_show_labels(&self) -> bool {
        self.show_labels || self.color_mode == ComparisonColorMode::Single
    }

    pub fn series_render_color(&self, theme: &Theme, series: &Series) -> Color {
        match self.color_mode {
            ComparisonColorMode::Multi => series.color,
            ComparisonColorMode::Single => Self::single_color(theme),
        }
    }

    pub fn apply_style_colors(&mut self, theme: &Theme) {
        match self.color_mode {
            ComparisonColorMode::Multi => {
                let colors = series_colors(theme);
                for (idx, series) in self.series.iter_mut().enumerate() {
                    series.color = colors[idx % colors.len()];
                }
            }
            ComparisonColorMode::Single => {
                let color = Self::single_color(theme);
                for series in &mut self.series {
                    series.color = color;
                }
            }
        }
        self.cache.clear();
    }

    pub fn single_color(theme: &Theme) -> Color {
        theme.palette().primary
    }

    pub(crate) fn set_dotted_background(&mut self, enabled: bool, opacity: f32) {
        if self.dotted_background != enabled
            || (self.dotted_background_opacity - opacity).abs() > f32::EPSILON
        {
            self.dotted_background = enabled;
            self.dotted_background_opacity = opacity;
            self.cache.clear();
        }
    }

    pub(crate) fn set_gradient_background(&mut self, enabled: bool) {
        if self.gradient_background != enabled {
            self.gradient_background = enabled;
            self.cache.clear();
        }
    }

    pub(crate) fn set_hollow_candle_mode(&mut self, mode: crate::config::ChartHollowCandleMode) {
        if self.hollow_candle_mode != mode {
            self.hollow_candle_mode = mode;
            self.cache.clear();
        }
    }

    pub(crate) fn set_crosshair_style(&mut self, style: crate::config::ChartCrosshairStyle) {
        let style = style.normalized();
        if self.crosshair_style != style {
            self.crosshair_style = style;
            self.cache.clear();
        }
    }

    pub(crate) fn set_crosshair_guides_enabled(&mut self, enabled: bool) {
        if self.crosshair_guides_enabled != enabled {
            self.crosshair_guides_enabled = enabled;
            self.cache.clear();
        }
    }

    pub(crate) fn set_crosshair_scale(&mut self, scale: f32) {
        let scale = crate::config::normalize_chart_crosshair_scale(scale);
        if (self.crosshair_scale - scale).abs() > f32::EPSILON {
            self.crosshair_scale = scale;
            self.cache.clear();
        }
    }

    /// Push or update a candle for a specific series, maintaining sorted order.
    pub fn push_candle(&mut self, symbol: &str, candle: Candle) {
        if !is_valid_candle(&candle) {
            return;
        }
        if let Some(s) = self.series.iter_mut().find(|s| s.symbol == symbol) {
            let ts = candle.open_time;
            // Check the common case: candle is the latest or updates the last
            if let Some(last) = s.candles.last_mut() {
                if last.open_time == ts {
                    *last = candle;
                } else if last.open_time < ts {
                    s.candles.push(candle);
                } else {
                    // Out-of-order: insert at correct sorted position
                    match s.candles.binary_search_by_key(&ts, |c| c.open_time) {
                        Ok(i) => s.candles[i] = candle,
                        Err(i) => s.candles.insert(i, candle),
                    }
                }
            } else {
                s.candles.push(candle);
            }
            self.cache.clear();
        }
    }
}

#[cfg(test)]
mod tests;
