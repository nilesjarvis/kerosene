use crate::spaghetti::ComparisonColorMode;
use serde::{Deserialize, Serialize};

use super::default_detached_chart_window_height;
use super::default_detached_chart_window_width;
use crate::config::default_timeframe;

/// Persisted state for a detached comparison chart window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetachedSpaghettiWindowConfig {
    /// Spaghetti chart instance ID shown in the detached window.
    #[serde(default)]
    pub chart_id: u64,
    /// Last window width in logical pixels.
    #[serde(default = "default_detached_chart_window_width")]
    pub width: f32,
    /// Last window height in logical pixels.
    #[serde(default = "default_detached_chart_window_height")]
    pub height: f32,
    /// Last window X position.
    #[serde(default)]
    pub x: Option<f32>,
    /// Last window Y position.
    #[serde(default)]
    pub y: Option<f32>,
}

/// Persisted state for a comparison chart pane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpaghettiChartConfig {
    /// Stable spaghetti chart pane ID.
    #[serde(default)]
    pub id: u64,
    /// Symbol keys for the series (e.g. ["BTC", "ETH", "SOL"]).
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Chart timeframe.
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    /// Whether this chart is pair-ratio mode.
    #[serde(default)]
    pub pair_mode: bool,
    /// Pair chart render mode: true = candlesticks, false = line.
    #[serde(default)]
    pub pair_candle_mode: bool,
    /// Comparison chart line color mode.
    #[serde(default)]
    pub color_mode: ComparisonColorMode,
    /// Whether comparison chart ticker labels are shown.
    #[serde(default)]
    pub show_labels: bool,
    /// Optional comparison anchor (e.g. "us", "utc_week").
    #[serde(default)]
    pub anchor: Option<String>,
    /// Optional manual granularity while anchor mode is active (e.g. "M5").
    #[serde(default)]
    pub anchor_granularity: Option<String>,
}

impl SpaghettiChartConfig {
    #[inline]
    pub(crate) fn empty(id: u64) -> Self {
        Self {
            id,
            symbols: Vec::new(),
            timeframe: default_timeframe(),
            pair_mode: false,
            pair_candle_mode: false,
            color_mode: ComparisonColorMode::default(),
            show_labels: false,
            anchor: None,
            anchor_granularity: None,
        }
    }
}
