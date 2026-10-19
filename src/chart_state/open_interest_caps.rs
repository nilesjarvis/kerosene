use crate::app_state::TradingTerminal;
use std::collections::{HashMap, HashSet};

pub(crate) const OPEN_INTEREST_CAP_REFRESH_MS: u64 = 60_000;
const OPEN_INTEREST_CAP_MAX_AGE_MS: u64 = 2 * OPEN_INTEREST_CAP_REFRESH_MS;

/// Public limits and status shared across chart instances, independent of live asset contexts.
#[derive(Default)]
pub(crate) struct ChartOpenInterestCaps {
    pub(crate) dexes: HashMap<String, DexOpenInterestCaps>,
}

#[derive(Default)]
pub(crate) struct DexOpenInterestCaps {
    pub(crate) caps: HashMap<String, f64>,
    pub(crate) updated_at_ms: Option<u64>,
    pub(crate) in_flight: bool,
    pub(crate) next_attempt_ms: u64,
    pub(crate) failures: u8,
    pub(crate) status: DexOpenInterestCapStatus,
}

/// Refreshed independently so a missing numeric limit cannot suppress a warning.
#[derive(Default)]
pub(crate) struct DexOpenInterestCapStatus {
    pub(crate) coins: HashSet<String>,
    pub(crate) updated_at_ms: Option<u64>,
    pub(crate) in_flight: bool,
    pub(crate) next_attempt_ms: u64,
    pub(crate) failures: u8,
}

impl TradingTerminal {
    pub(crate) fn chart_open_interest_cap(&self, symbol: &str, now_ms: u64) -> Option<f64> {
        let (dex, _) = symbol.split_once(':')?;
        let entry = self.chart_open_interest_caps.dexes.get(dex)?;
        if now_ms.saturating_sub(entry.updated_at_ms?) >= OPEN_INTEREST_CAP_MAX_AGE_MS {
            return None;
        }
        entry.caps.get(symbol).copied()
    }

    /// `None` means unknown or stale; only a fresh exchange response confirms status.
    pub(crate) fn chart_is_at_open_interest_cap(&self, symbol: &str, now_ms: u64) -> Option<bool> {
        if !self.is_perp_coin(symbol) {
            return None;
        }
        let dex = symbol.split_once(':').map_or("", |(dex, _)| dex);
        let status = &self.chart_open_interest_caps.dexes.get(dex)?.status;
        if now_ms.saturating_sub(status.updated_at_ms?) >= OPEN_INTEREST_CAP_MAX_AGE_MS {
            return None;
        }
        Some(status.coins.contains(symbol))
    }
}
