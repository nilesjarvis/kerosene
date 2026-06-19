use serde::de;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub enum OrderBookSymbolModeConfig {
    #[default]
    Active,
    Fixed(String),
}

impl OrderBookSymbolModeConfig {
    fn fallback_config_value() -> &'static str {
        "Active"
    }
}

impl<'de> Deserialize<'de> for OrderBookSymbolModeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(value) if value == Self::fallback_config_value() => {
                Ok(Self::Active)
            }
            serde_json::Value::String(value) => {
                push_unknown_order_book_config_warning(
                    "symbol mode",
                    &value,
                    Self::fallback_config_value(),
                );
                Ok(Self::default())
            }
            serde_json::Value::Object(mut object) if object.len() == 1 => {
                if let Some(fixed) = object.remove("Fixed") {
                    let symbol = fixed.as_str().ok_or_else(|| {
                        de::Error::custom("order book Fixed symbol mode must contain a string")
                    })?;
                    Ok(Self::Fixed(symbol.to_string()))
                } else {
                    let value = serde_json::Value::Object(object).to_string();
                    push_unknown_order_book_config_warning(
                        "symbol mode",
                        &value,
                        Self::fallback_config_value(),
                    );
                    Ok(Self::default())
                }
            }
            value => {
                let value = value.to_string();
                push_unknown_order_book_config_warning(
                    "symbol mode",
                    &value,
                    Self::fallback_config_value(),
                );
                Ok(Self::default())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
pub enum OrderBookDisplayModeConfig {
    #[default]
    DepthList,
    DomLadder,
    DepthChart,
}

impl OrderBookDisplayModeConfig {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "DepthList" => Some(Self::DepthList),
            "DomLadder" => Some(Self::DomLadder),
            "DepthChart" => Some(Self::DepthChart),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::DepthList => "DepthList",
            Self::DomLadder => "DomLadder",
            Self::DepthChart => "DepthChart",
        }
    }
}

impl<'de> Deserialize<'de> for OrderBookDisplayModeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            push_unknown_order_book_config_warning("display mode", &value, default.config_value());
            default
        }))
    }
}

fn push_unknown_order_book_config_warning(field: &str, value: &str, fallback: &str) {
    crate::config::push_config_warning(format!(
        "Unknown order book {field} {value:?} in config; using {fallback}"
    ));
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
