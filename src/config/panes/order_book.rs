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
    DepthChart,
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
    // Centered is the default for configs that predate the field; layouts
    // saved since then carry the user's explicit choice.
    #[serde(default = "default_center_on_mid")]
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

fn default_center_on_mid() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_on_mid_defaults_to_true_for_configs_predating_the_field() {
        let cfg: OrderBookConfig =
            serde_json::from_str(r#"{"id": 3}"#).expect("minimal config should deserialize");
        assert!(cfg.center_on_mid);
    }

    #[test]
    fn center_on_mid_keeps_the_persisted_value() {
        let cfg: OrderBookConfig = serde_json::from_str(r#"{"id": 3, "center_on_mid": false}"#)
            .expect("explicit config should deserialize");
        assert!(!cfg.center_on_mid);
    }
}
