use crate::config::listings::{ListingsHistory, MAX_LISTING_EVENTS};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, io::Read, path::Path};

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    history: ListingsHistory,
}

pub(crate) fn load() -> Result<ListingsHistory, String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(ListingsHistory::default());
    };
    load_from(&root.join("listings.json"))
}

fn load_from(path: &Path) -> Result<ListingsHistory, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ListingsHistory::default());
        }
        Err(_) => return Err("Listing history could not be read".into()),
    };
    let envelope: Envelope = serde_json::from_reader(file.take(4 * 1024 * 1024))
        .map_err(|_| "Invalid listing history".to_string())?;
    if envelope.version != 1 {
        return Err("Unsupported listing history version".into());
    }
    let mut history = envelope.history;
    history
        .events
        .sort_by_key(|event| Reverse(event.detected_at_ms));
    history.events.truncate(MAX_LISTING_EVENTS);
    Ok(history)
}

pub(crate) async fn save(history: ListingsHistory) -> Result<(), String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(());
    };
    tokio::task::spawn_blocking(move || save_to(&root.join("listings.json"), history))
        .await
        .map_err(|_| "Listing history save interrupted".to_string())?
}

fn save_to(path: &Path, history: ListingsHistory) -> Result<(), String> {
    let bytes = serde_json::to_vec(&Envelope {
        version: 1,
        history,
    })
    .map_err(|_| "Listing history could not be encoded".to_string())?;
    crate::api_cache::write_bytes_atomic(path, &bytes)
        .map_err(|_| "Listing history could not be saved".to_string())
}

#[cfg(test)]
mod tests;
