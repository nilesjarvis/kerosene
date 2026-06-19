use serde::{Deserialize, Deserializer, Serialize};

// ---------------------------------------------------------------------------
// Chart Candle Data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
pub enum ReadDataProvider {
    #[default]
    Hyperliquid,
    Hydromancer,
}

impl<'de> Deserialize<'de> for ReadDataProvider {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "Hyperliquid" => Ok(Self::Hyperliquid),
            "Hydromancer" => Ok(Self::Hydromancer),
            unknown => {
                let default = Self::default();
                crate::config::push_config_warning(format!(
                    "Unknown read data provider {unknown:?} in config; using {}",
                    default.label()
                ));
                Ok(default)
            }
        }
    }
}

impl ReadDataProvider {
    pub const ALL: [Self; 2] = [Self::Hyperliquid, Self::Hydromancer];

    pub fn label(self) -> &'static str {
        match self {
            Self::Hyperliquid => "Hyperliquid",
            Self::Hydromancer => "Hydromancer",
        }
    }

    pub fn chart_backfill_source(self) -> ChartBackfillSource {
        match self {
            Self::Hyperliquid => ChartBackfillSource::Hyperliquid,
            Self::Hydromancer => ChartBackfillSource::Hydromancer,
        }
    }
}

impl std::fmt::Display for ReadDataProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ChartBackfillSource {
    #[default]
    Hyperliquid,
    Hydromancer,
}

impl ChartBackfillSource {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Hyperliquid" => Some(Self::Hyperliquid),
            "Hydromancer" => Some(Self::Hydromancer),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hyperliquid => "Hyperliquid",
            Self::Hydromancer => "Hydromancer",
        }
    }
}

impl<'de> Deserialize<'de> for ChartBackfillSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            crate::config::push_config_warning(format!(
                "Unknown chart backfill source {value:?} in config; using {}",
                default.label()
            ));
            default
        }))
    }
}

impl std::fmt::Display for ChartBackfillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Chart Candle Appearance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ChartHollowCandleMode {
    #[default]
    Off,
    Up,
    Down,
    Both,
}

impl ChartHollowCandleMode {
    pub const ALL: [Self; 4] = [Self::Off, Self::Up, Self::Down, Self::Both];

    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Off" => Some(Self::Off),
            "Up" => Some(Self::Up),
            "Down" => Some(Self::Down),
            "Both" => Some(Self::Both),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Both => "Both",
        }
    }

    pub fn applies_to(self, bullish: bool) -> bool {
        match self {
            Self::Off => false,
            Self::Up => bullish,
            Self::Down => !bullish,
            Self::Both => true,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Up => "Up candles",
            Self::Down => "Down candles",
            Self::Both => "Up and down candles",
        }
    }
}

impl<'de> Deserialize<'de> for ChartHollowCandleMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            push_unknown_chart_appearance_warning(
                "hollow candle mode",
                &value,
                default.config_value(),
            );
            default
        }))
    }
}

impl std::fmt::Display for ChartHollowCandleMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// How the main price series renders: traditional candlesticks, or a single
/// close-price line with a gradient area fill beneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ChartSeriesStyle {
    #[default]
    Candles,
    Line,
}

impl ChartSeriesStyle {
    pub const ALL: [Self; 2] = [Self::Candles, Self::Line];

    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "Candles" => Some(Self::Candles),
            "Line" => Some(Self::Line),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Candles => "Candles",
            Self::Line => "Line",
        }
    }

    pub fn is_line(self) -> bool {
        matches!(self, Self::Line)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Candles => "Candlesticks",
            Self::Line => "Line",
        }
    }
}

impl<'de> Deserialize<'de> for ChartSeriesStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            push_unknown_chart_appearance_warning("series style", &value, default.config_value());
            default
        }))
    }
}

impl std::fmt::Display for ChartSeriesStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

fn push_unknown_chart_appearance_warning(field: &str, value: &str, fallback: &str) {
    crate::config::push_config_warning(format!(
        "Unknown chart {field} {value:?} in config; using {fallback}"
    ));
}

#[cfg(test)]
mod series_style_tests {
    use super::ChartSeriesStyle;

    #[test]
    fn series_style_defaults_to_candles_and_reports_line() {
        assert_eq!(ChartSeriesStyle::default(), ChartSeriesStyle::Candles);
        assert!(!ChartSeriesStyle::Candles.is_line());
        assert!(ChartSeriesStyle::Line.is_line());
        assert_eq!(
            ChartSeriesStyle::ALL,
            [ChartSeriesStyle::Candles, ChartSeriesStyle::Line]
        );
        assert_eq!(ChartSeriesStyle::Line.label(), "Line");
    }
}
