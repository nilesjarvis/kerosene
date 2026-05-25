use serde::{Deserialize, Serialize};

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
    pub center_on_mid: bool,
    #[serde(default)]
    pub reverse_side: bool,
    #[serde(default)]
    pub show_spread_chart: bool,
    #[serde(default = "default_spread_chart_height")]
    pub spread_chart_height: f32,
}

fn default_spread_chart_height() -> f32 {
    60.0
}
