use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, fmt};

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
    next_snapshot_request, snapshot_request_for_timeframe, unavailable_snapshot,
};
pub(crate) use state::{
    DEFAULT_JOURNAL_WINDOW_HEIGHT, DEFAULT_JOURNAL_WINDOW_WIDTH, JournalAccountState,
};
pub use state::{JournalFilter, JournalSort, JournalState, JournalSyncStatus};

#[derive(Clone, Default, Serialize)]
pub struct JournalNote {
    pub open: String,
    pub close: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl JournalNote {
    /// True when the reflection carries no thesis, no reflection, and no tags.
    pub fn is_empty(&self) -> bool {
        self.open.trim().is_empty() && self.close.trim().is_empty() && self.tags.is_empty()
    }
}

impl fmt::Debug for JournalNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalNote")
            .field("open", &format_args!("len={}", self.open.len()))
            .field("close", &format_args!("len={}", self.close.len()))
            .field("tags", &format_args!("len={}", self.tags.len()))
            .finish()
    }
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
                #[serde(default)]
                tags: Vec<String>,
            },
            Legacy(String),
        }

        match NoteWrapper::deserialize(deserializer)? {
            NoteWrapper::Structured { open, close, tags } => Ok(JournalNote { open, close, tags }),
            NoteWrapper::Legacy(s) => Ok(JournalNote {
                open: s,
                close: String::new(),
                tags: Vec::new(),
            }),
        }
    }
}

/// Parse a free-form tag input ("#breakout, momentum trend") into normalized,
/// de-duplicated tags. Leading `#` and surrounding whitespace are stripped;
/// order is preserved and case is kept as typed.
pub fn parse_journal_tags(raw: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for token in raw.split([',', ' ', '\t', '\n', '#']) {
        let tag = token.trim();
        if tag.is_empty() {
            continue;
        }
        if !tags
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(tag))
        {
            tags.push(tag.to_string());
        }
    }
    tags
}

/// Render tags back into an editable input string (space-separated, no `#`).
pub fn journal_tags_input(tags: &[String]) -> String {
    tags.join(" ")
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
