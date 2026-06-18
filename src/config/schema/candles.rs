use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Chart Candle Data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReadDataProvider {
    #[default]
    Hyperliquid,
    Hydromancer,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChartBackfillSource {
    #[default]
    Hyperliquid,
    Hydromancer,
}

impl ChartBackfillSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hyperliquid => "Hyperliquid",
            Self::Hydromancer => "Hydromancer",
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChartHollowCandleMode {
    #[default]
    Off,
    Up,
    Down,
    Both,
}

impl ChartHollowCandleMode {
    pub const ALL: [Self; 4] = [Self::Off, Self::Up, Self::Down, Self::Both];

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

impl std::fmt::Display for ChartHollowCandleMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// How the main price series renders: traditional candlesticks, or a single
/// close-price line with a gradient area fill beneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChartSeriesStyle {
    #[default]
    Candles,
    Line,
}

impl ChartSeriesStyle {
    pub const ALL: [Self; 2] = [Self::Candles, Self::Line];

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

impl std::fmt::Display for ChartSeriesStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
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
