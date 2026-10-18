use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

pub(crate) const MAX_UPDATE_EVENTS: usize = 500;
pub(crate) const MAX_UPDATE_ASSETS: usize = 20_000;

/// Canonical nonnegative decimal; comparisons never round API values through f64.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct ParameterDecimal(String);

impl ParameterDecimal {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ParameterDecimal {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (whole, fraction) = value.split_once('.').unwrap_or((&value, ""));
        if value.len() > 64
            || whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("Invalid parameter decimal".into());
        }
        let whole = whole.trim_start_matches('0');
        let whole = if whole.is_empty() { "0" } else { whole };
        let fraction = fraction.trim_end_matches('0');
        Ok(Self(if fraction.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{fraction}")
        }))
    }
}

impl From<ParameterDecimal> for String {
    fn from(value: ParameterDecimal) -> Self {
        value.0
    }
}

impl Ord for ParameterDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let digits = |value: &str| value.find('.').unwrap_or(value.len());
        digits(&self.0)
            .cmp(&digits(&other.0))
            .then_with(|| self.0.cmp(&other.0))
    }
}

impl PartialOrd for ParameterDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MarginTier {
    pub lower_bound: ParameterDecimal,
    pub max_leverage: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MarketMarginMode {
    Normal,
    NoCross,
    StrictIsolated,
}

impl MarketMarginMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Cross / isolated",
            Self::NoCross => "Isolated only",
            Self::StrictIsolated => "Strict isolated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AssetParameters {
    pub max_leverage: u32,
    pub margin_tiers: Vec<MarginTier>,
    /// Legacy onlyIsolated=true cannot distinguish noCross from strictIsolated.
    pub margin_mode: Option<MarketMarginMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParameterKind {
    OiCap,
    MaxLeverage,
    MarginTiers,
    MarginMode,
}

impl ParameterKind {
    pub const ALL: [Self; 4] = [
        Self::OiCap,
        Self::MaxLeverage,
        Self::MarginTiers,
        Self::MarginMode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OiCap => "OI cap",
            Self::MaxLeverage => "Maximum leverage",
            Self::MarginTiers => "Margin tiers",
            Self::MarginMode => "Margin mode",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParameterChange {
    OiCap {
        old: ParameterDecimal,
        new: ParameterDecimal,
    },
    MaxLeverage {
        old: u32,
        new: u32,
    },
    MarginTiers {
        old: Vec<MarginTier>,
        new: Vec<MarginTier>,
    },
    MarginMode {
        old: MarketMarginMode,
        new: MarketMarginMode,
    },
}

impl ParameterChange {
    pub fn kind(&self) -> ParameterKind {
        match self {
            Self::OiCap { .. } => ParameterKind::OiCap,
            Self::MaxLeverage { .. } => ParameterKind::MaxLeverage,
            Self::MarginTiers { .. } => ParameterKind::MarginTiers,
            Self::MarginMode { .. } => ParameterKind::MarginMode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ParameterEvent {
    pub key: String,
    pub detected_at_ms: u64,
    pub previous_observed_at_ms: u64,
    pub after_gap: bool,
    pub changes: Vec<ParameterChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UpdatesHistory {
    pub parameters: BTreeMap<String, AssetParameters>,
    pub caps: BTreeMap<String, ParameterDecimal>,
    pub parameters_observed_at_ms: Option<u64>,
    pub caps_observed_at_ms: Option<u64>,
    pub events: Vec<ParameterEvent>,
}

#[cfg(test)]
mod tests;
