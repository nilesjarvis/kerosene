use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

mod aggregation;
mod cache;
mod state;

pub use aggregation::{
    AggregatedTrade, aggregate_trades_with_diagnostics, merge_fills, newest_fill_time,
    normalize_fills,
};
pub use cache::{load_cache, save_cache};
#[cfg(test)]
pub use state::JournalAccountState;
pub use state::{JournalFilter, JournalSort, JournalState};

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
