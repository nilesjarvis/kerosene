// ---------------------------------------------------------------------------
// Chart timeframe model
// ---------------------------------------------------------------------------

mod metadata;

#[cfg(test)]
mod tests;

use metadata::{ALL_TIMEFRAMES, API_STRS, CONFIG_STRS, DURATIONS_MS, LABELS, LOOKBACKS_MS};

/// Supported candlestick intervals from the Hyperliquid API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub(crate) enum Timeframe {
    M1,
    M3,
    M5,
    M15,
    M30,
    H1,
    H2,
    H4,
    H8,
    H12,
    D1,
    D3,
    W1,
    Mo1,
}

impl Timeframe {
    pub(crate) fn from_config_str_opt(s: &str) -> Option<Self> {
        CONFIG_STRS
            .iter()
            .position(|config_str| *config_str == s)
            .map(|idx| ALL_TIMEFRAMES[idx])
    }

    pub(crate) fn from_config_str(s: &str) -> Self {
        Self::from_config_str_opt(s).unwrap_or(Timeframe::H1)
    }

    pub(crate) fn config_str(self) -> &'static str {
        CONFIG_STRS[self.index()]
    }

    /// The interval string expected by the API.
    pub(crate) fn api_str(self) -> &'static str {
        API_STRS[self.index()]
    }

    pub(crate) fn duration_ms(self) -> u64 {
        DURATIONS_MS[self.index()]
    }

    /// Short label for the UI button.
    pub(crate) fn label(self) -> &'static str {
        LABELS[self.index()]
    }

    /// How far back to fetch (in milliseconds) so the chart has enough data.
    /// Aims for roughly 200-500 candles worth of history.
    pub(crate) fn lookback_ms(self) -> u64 {
        LOOKBACKS_MS[self.index()]
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Timeframes shown in the chart toolbar.
pub(crate) const TIMEFRAME_OPTIONS: &[Timeframe] = &[
    Timeframe::M1,
    Timeframe::M5,
    Timeframe::M15,
    Timeframe::H1,
    Timeframe::H4,
    Timeframe::D1,
    Timeframe::W1,
];
