use crate::annotations::AnnotationConfig;
use serde::{Deserialize, Serialize};

use super::super::{default_symbol, default_timeframe};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacroIndicatorsConfig {
    #[serde(default)]
    pub tf_sma_50: bool,
    #[serde(default)]
    pub tf_ema_50: bool,
    #[serde(default)]
    pub tf_sma_200: bool,
    #[serde(default)]
    pub tf_ema_200: bool,
    #[serde(default)]
    pub sma_50d: bool,
    #[serde(default)]
    pub ema_50d: bool,
    #[serde(default)]
    pub sma_200d: bool,
    #[serde(default)]
    pub ema_200d: bool,
    #[serde(default)]
    pub sma_20w: bool,
    #[serde(default)]
    pub ema_20w: bool,
    #[serde(default)]
    pub sma_50w: bool,
    #[serde(default)]
    pub ema_50w: bool,
    #[serde(default)]
    pub sma_12m: bool,
    #[serde(default)]
    pub ema_12m: bool,
    #[serde(default)]
    pub show_funding_rate: bool,
    #[serde(default)]
    pub show_volume_profile: bool,
    #[serde(default = "default_true")]
    pub show_labels: bool,
}

impl Default for MacroIndicatorsConfig {
    fn default() -> Self {
        Self {
            tf_sma_50: false,
            tf_ema_50: false,
            tf_sma_200: false,
            tf_ema_200: false,
            sma_50d: false,
            ema_50d: false,
            sma_200d: false,
            ema_200d: false,
            sma_20w: false,
            ema_20w: false,
            sma_50w: false,
            ema_50w: false,
            sma_12m: false,
            ema_12m: false,
            show_funding_rate: false,
            show_volume_profile: false,
            show_labels: true,
        }
    }
}

/// Persisted state for a single chart pane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartConfig {
    /// Stable chart pane ID.
    #[serde(default)]
    pub id: u64,
    /// Symbol key (e.g. "HYPE", "BTC", "xyz:NVDA").
    #[serde(default = "default_symbol")]
    pub symbol: String,
    /// Chart timeframe config string (e.g. "H1", "M5").
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    /// User-drawn annotations on this chart.
    #[serde(default)]
    pub annotations: Vec<AnnotationConfig>,
    /// Whether chart price axis is visually inverted.
    #[serde(default)]
    pub inverted: bool,
    /// Whether user fills are shown as buy/sell trade dots.
    #[serde(default)]
    pub show_trade_markers: bool,
    /// Whether the chart header is collapsed to a ticker-only strip.
    #[serde(default)]
    pub header_collapsed: bool,
    /// Desired funding-rate sub-panel height in pixels.
    #[serde(default = "default_funding_panel_height")]
    pub funding_panel_height: u16,
    /// Active macro timeframe moving averages
    #[serde(default)]
    pub macro_indicators: MacroIndicatorsConfig,
    /// Whether open interest is displayed as USD notional instead of coin amount.
    #[serde(default)]
    pub open_interest_as_notional: bool,
    /// Whether HIP-4 volume is displayed as USD notional instead of contracts.
    #[serde(default)]
    pub outcome_volume_as_notional: bool,
}

impl ChartConfig {
    #[inline]
    pub(crate) fn empty(id: u64, symbol: impl Into<String>, timeframe: impl Into<String>) -> Self {
        Self {
            id,
            symbol: symbol.into(),
            timeframe: timeframe.into(),
            annotations: Vec::new(),
            inverted: false,
            show_trade_markers: false,
            header_collapsed: false,
            funding_panel_height: default_funding_panel_height(),
            macro_indicators: MacroIndicatorsConfig::default(),
            open_interest_as_notional: false,
            outcome_volume_as_notional: false,
        }
    }
}

/// Persisted state for a detached candlestick chart window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetachedChartWindowConfig {
    /// Chart instance shown in the detached window.
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

fn default_true() -> bool {
    true
}

fn default_funding_panel_height() -> u16 {
    56
}

pub fn default_detached_chart_window_width() -> f32 {
    1100.0
}

pub fn default_detached_chart_window_height() -> f32 {
    720.0
}
