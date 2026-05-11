use crate::annotations::AnnotationConfig;
use crate::spaghetti::ComparisonColorMode;
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
    #[serde(default)]
    pub show_funding_rate: bool,
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
    /// Desired funding-rate sub-panel height in pixels.
    #[serde(default = "default_funding_panel_height")]
    pub funding_panel_height: u16,
    /// Active macro timeframe moving averages
    #[serde(default)]
    pub macro_indicators: MacroIndicatorsConfig,
    /// Whether open interest is displayed as USD notional instead of coin amount.
    #[serde(default)]
    pub open_interest_as_notional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OrderBookSymbolModeConfig {
    #[default]
    Active,
    Fixed(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OrderBookDisplayModeConfig {
    #[default]
    DepthList,
    DomLadder,
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
    pub display_mode: OrderBookDisplayModeConfig,
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

fn default_true() -> bool {
    true
}

fn default_funding_panel_height() -> u16 {
    56
}

fn default_spread_chart_height() -> f32 {
    60.0
}

#[cfg(test)]
mod tests {
    use super::{OrderBookConfig, OrderBookDisplayModeConfig};

    #[test]
    fn order_book_config_defaults_to_depth_list_display_mode() {
        let config: OrderBookConfig =
            serde_json::from_str(r#"{"id":7,"tick_size":1.0}"#).expect("config");

        assert_eq!(config.display_mode, OrderBookDisplayModeConfig::DepthList);
    }

    #[test]
    fn order_book_config_round_trips_dom_ladder_display_mode() {
        let config: OrderBookConfig =
            serde_json::from_str(r#"{"id":7,"tick_size":1.0,"display_mode":"DomLadder"}"#)
                .expect("config");

        let rendered = serde_json::to_string(&config).expect("json");
        assert!(rendered.contains(r#""display_mode":"DomLadder""#));
    }
}
