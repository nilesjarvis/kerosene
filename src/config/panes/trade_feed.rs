use super::OrderBookSymbolModeConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TradeFeedConfig {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub mode: OrderBookSymbolModeConfig,
    #[serde(default)]
    pub group_trades: bool,
    #[serde(default)]
    pub min_group_notional: f64,
    #[serde(default)]
    pub size_in_notional: bool,
    #[serde(default)]
    pub show_context: bool,
}
