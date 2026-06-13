use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

mod aggregation;
mod cache;
mod current_positions;
mod snapshot;
mod state;

pub use aggregation::{
    AggregatedTrade, JournalAttributedFillRole, JournalTradeDetails,
    aggregate_trades_with_diagnostics, merge_fills, newest_fill_time, normalize_fills,
};
#[cfg(test)]
pub use aggregation::{FillIdentity, JournalAttributedFill};
pub use cache::{clear_cache, load_cache, save_cache};
pub use current_positions::{
    JournalPositionReconciliation, current_position_fallback_warning,
    reconcile_current_position_trades,
};
pub use snapshot::{
    JournalTradeSnapshot, JournalTradeSnapshotMetrics, JournalTradeSnapshotRequest,
    JournalTradeSnapshotStatus, build_journal_trade_snapshot, initial_snapshot_request,
    next_snapshot_request, unavailable_snapshot,
};
pub(crate) use state::{
    DEFAULT_JOURNAL_WINDOW_HEIGHT, DEFAULT_JOURNAL_WINDOW_WIDTH, JournalAccountState,
};
pub use state::{JournalFilter, JournalSort, JournalState, JournalSyncStatus};

#[derive(Debug, Clone, Default, Serialize)]
pub struct JournalNote {
    pub open: String,
    pub close: String,
}

impl<'de> Deserialize<'de> for JournalNote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum NoteWrapper {
            Structured {
                #[serde(default)]
                open: String,
                #[serde(default)]
                close: String,
            },
            Legacy(String),
        }

        match NoteWrapper::deserialize(deserializer)? {
            NoteWrapper::Structured { open, close } => Ok(JournalNote { open, close }),
            NoteWrapper::Legacy(s) => Ok(JournalNote {
                open: s,
                close: String::new(),
            }),
        }
    }
}

pub fn note_key_for_trade(
    entries: &HashMap<String, JournalNote>,
    trade: &AggregatedTrade,
) -> Option<String> {
    if entries.contains_key(&trade.id) {
        return Some(trade.id.clone());
    }

    trade
        .legacy_note_ids
        .iter()
        .find(|id| entries.contains_key(*id))
        .cloned()
}

pub fn note_for_trade<'a>(
    entries: &'a HashMap<String, JournalNote>,
    trade: &AggregatedTrade,
) -> Option<&'a JournalNote> {
    let key = note_key_for_trade(entries, trade)?;
    entries.get(&key)
}

#[cfg(test)]
mod tests;
