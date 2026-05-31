use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Chart Candle Data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChartBackfillSource {
    #[default]
    Hyperliquid,
    Hydromancer,
}

impl ChartBackfillSource {
    pub const ALL: [Self; 2] = [Self::Hyperliquid, Self::Hydromancer];

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
