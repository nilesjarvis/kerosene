use crate::annotations::AnnotationConfig;
use serde::{Deserialize, Serialize};

use super::{default_symbol, default_timeframe};

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
    /// Active macro timeframe moving averages
    #[serde(default)]
    pub macro_indicators: MacroIndicatorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OrderBookSymbolModeConfig {
    #[default]
    Active,
    Fixed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderBookConfig {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub mode: OrderBookSymbolModeConfig,
    #[serde(default)]
    pub tick_size: f64,
    #[serde(default)]
    pub show_spread_chart: bool,
    #[serde(default = "default_spread_chart_height")]
    pub spread_chart_height: f32,
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
    /// Whether this chart is pair-trade mode.
    #[serde(default)]
    pub pair_mode: bool,
    /// Pair notional per leg.
    #[serde(default = "default_pair_notional")]
    pub pair_notional: String,
    /// Pair chart render mode: true = candlesticks, false = line.
    #[serde(default)]
    pub pair_candle_mode: bool,
    /// Optional comparison anchor (e.g. "us", "utc_week").
    #[serde(default)]
    pub anchor: Option<String>,
    /// Optional manual granularity while anchor mode is active (e.g. "M5").
    #[serde(default)]
    pub anchor_granularity: Option<String>,
}

pub fn default_pair_notional() -> String {
    "100".to_string()
}

fn default_true() -> bool {
    true
}

fn default_spread_chart_height() -> f32 {
    60.0
}
