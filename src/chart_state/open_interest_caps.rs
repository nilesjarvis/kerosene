use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::pane_state::PaneKind;
use std::collections::{BTreeSet, HashMap, HashSet};

pub(crate) const OPEN_INTEREST_CAP_REFRESH_MS: u64 = 60_000;
const OPEN_INTEREST_CAP_MAX_AGE_MS: u64 = 2 * OPEN_INTEREST_CAP_REFRESH_MS;
pub(crate) const OPEN_INTEREST_CAP_STREAM_MAX_AGE_MS: u64 = 15_000;

/// Only public cap lists are retained from the aggregate websocket response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenInterestCapSnapshot {
    pub(crate) coins: HashSet<String>,
    pub(crate) server_time_ms: u64,
    pub(crate) received_at_ms: u64,
}

impl OpenInterestCapSnapshot {
    pub(crate) fn is_fresh(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.received_at_ms) < OPEN_INTEREST_CAP_STREAM_MAX_AGE_MS
            && now_ms.saturating_sub(self.server_time_ms) < OPEN_INTEREST_CAP_STREAM_MAX_AGE_MS
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenInterestCapStreamEvent {
    Snapshot(OpenInterestCapSnapshot),
    Unavailable,
}

/// Public limits and status shared across chart instances, independent of live asset contexts.
#[derive(Default)]
pub(crate) struct ChartOpenInterestCaps {
    pub(crate) dexes: HashMap<String, DexOpenInterestCaps>,
    pub(crate) live: Option<OpenInterestCapSnapshot>,
    pub(crate) last_live_update_ms: Option<u64>,
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
    pub(crate) requested_at_ms: Option<u64>,
    pub(crate) next_attempt_ms: u64,
    pub(crate) failures: u8,
}

impl TradingTerminal {
    pub(crate) fn chart_open_interest_cap_dexes(&self) -> BTreeSet<String> {
        self.workspace_pane_kinds()
            .filter(|(workspace, _, _)| match workspace {
                WorkspaceId::Main => true,
                WorkspaceId::Canvas(id) => self
                    .canvases
                    .get(id)
                    .is_some_and(|canvas| canvas.window_id.is_some()),
            })
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::Chart(id) => Some(*id),
                _ => None,
            })
            .chain(
                self.detached_chart_windows
                    .values()
                    .map(|window| window.chart_id),
            )
            .filter_map(|id| self.charts.get(&id))
            .filter(|chart| {
                !chart.header_collapsed
                    && !chart.symbol.is_empty()
                    && self.is_perp_coin(&chart.symbol)
                    && !self.symbol_key_is_hidden(&chart.symbol)
            })
            .filter_map(|chart| match chart.symbol.split_once(':') {
                Some((dex, coin)) if !dex.is_empty() && !coin.is_empty() => Some(dex.to_string()),
                Some(_) => None,
                None => Some(String::new()),
            })
            .collect()
    }

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
        if let Some(snapshot) = &self.chart_open_interest_caps.live
            && snapshot.is_fresh(now_ms)
        {
            return Some(snapshot.coins.contains(symbol));
        }
        let dex = symbol.split_once(':').map_or("", |(dex, _)| dex);
        let status = &self.chart_open_interest_caps.dexes.get(dex)?.status;
        if now_ms.saturating_sub(status.updated_at_ms?) >= OPEN_INTEREST_CAP_MAX_AGE_MS {
            return None;
        }
        Some(status.coins.contains(symbol))
    }
}
