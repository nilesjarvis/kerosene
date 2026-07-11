use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderPreset {
    pub label: String,
    pub size: f64,
    #[serde(default)]
    pub price_offset_pct: Option<f64>,
}

impl fmt::Debug for OrderPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderPreset")
            .field("label", &format_args!("<redacted>"))
            .field("size", &format_args!("<redacted>"))
            .field(
                "price_offset_pct",
                &self.price_offset_pct.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderPresetsConfig {
    #[serde(default = "default_market_usd_presets")]
    pub market_usd: Vec<OrderPreset>,
    #[serde(default = "default_limit_usd_presets")]
    pub limit_usd: Vec<OrderPreset>,
    #[serde(default = "default_chase_usd_presets")]
    pub chase_usd: Vec<OrderPreset>,
    #[serde(default = "default_market_coin_presets")]
    pub market_coin: Vec<OrderPreset>,
    #[serde(default = "default_limit_coin_presets")]
    pub limit_coin: Vec<OrderPreset>,
    #[serde(default = "default_chase_coin_presets")]
    pub chase_coin: Vec<OrderPreset>,
}

impl fmt::Debug for OrderPresetsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderPresetsConfig")
            .field("market_usd_len", &self.market_usd.len())
            .field("limit_usd_len", &self.limit_usd.len())
            .field("chase_usd_len", &self.chase_usd.len())
            .field("market_coin_len", &self.market_coin.len())
            .field("limit_coin_len", &self.limit_coin.len())
            .field("chase_coin_len", &self.chase_coin.len())
            .finish()
    }
}

fn default_market_usd_presets() -> Vec<OrderPreset> {
    vec![
        OrderPreset {
            label: "$100".to_string(),
            size: 100.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "$500".to_string(),
            size: 500.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "$1k".to_string(),
            size: 1000.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "$5k".to_string(),
            size: 5000.0,
            price_offset_pct: None,
        },
    ]
}

fn default_limit_usd_presets() -> Vec<OrderPreset> {
    vec![
        OrderPreset {
            label: "-1%".to_string(),
            size: 500.0,
            price_offset_pct: Some(1.0),
        },
        OrderPreset {
            label: "-2%".to_string(),
            size: 1000.0,
            price_offset_pct: Some(2.0),
        },
        OrderPreset {
            label: "-5%".to_string(),
            size: 2000.0,
            price_offset_pct: Some(5.0),
        },
    ]
}

fn default_chase_usd_presets() -> Vec<OrderPreset> {
    vec![
        OrderPreset {
            label: "$500".to_string(),
            size: 500.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "$1k".to_string(),
            size: 1000.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "$5k".to_string(),
            size: 5000.0,
            price_offset_pct: None,
        },
    ]
}

fn default_market_coin_presets() -> Vec<OrderPreset> {
    vec![
        OrderPreset {
            label: "0.1".to_string(),
            size: 0.1,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "0.5".to_string(),
            size: 0.5,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "1.0".to_string(),
            size: 1.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "10.0".to_string(),
            size: 10.0,
            price_offset_pct: None,
        },
    ]
}

fn default_limit_coin_presets() -> Vec<OrderPreset> {
    vec![
        OrderPreset {
            label: "-1%".to_string(),
            size: 1.0,
            price_offset_pct: Some(1.0),
        },
        OrderPreset {
            label: "-2%".to_string(),
            size: 2.0,
            price_offset_pct: Some(2.0),
        },
        OrderPreset {
            label: "-5%".to_string(),
            size: 5.0,
            price_offset_pct: Some(5.0),
        },
    ]
}

fn default_chase_coin_presets() -> Vec<OrderPreset> {
    vec![
        OrderPreset {
            label: "1.0".to_string(),
            size: 1.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "5.0".to_string(),
            size: 5.0,
            price_offset_pct: None,
        },
        OrderPreset {
            label: "10.0".to_string(),
            size: 10.0,
            price_offset_pct: None,
        },
    ]
}

impl Default for OrderPresetsConfig {
    fn default() -> Self {
        Self {
            market_usd: default_market_usd_presets(),
            limit_usd: default_limit_usd_presets(),
            chase_usd: default_chase_usd_presets(),
            market_coin: default_market_coin_presets(),
            limit_coin: default_limit_coin_presets(),
            chase_coin: default_chase_coin_presets(),
        }
    }
}

#[cfg(test)]
mod tests;
