use crate::config::updates::{MAX_UPDATE_ASSETS, MAX_UPDATE_EVENTS, UpdatesHistory};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, io::Read, path::Path};

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    history: UpdatesHistory,
}

pub(crate) fn load() -> Result<UpdatesHistory, String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(UpdatesHistory::default());
    };
    load_from(&root.join("updates.json"))
}

fn load_from(path: &Path) -> Result<UpdatesHistory, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(UpdatesHistory::default());
        }
        Err(_) => return Err("Updates history could not be read".into()),
    };
    let envelope: Envelope = serde_json::from_reader(file.take(16 * 1024 * 1024))
        .map_err(|_| "Invalid updates history".to_string())?;
    if envelope.version != 1 {
        return Err("Unsupported updates history version".into());
    }
    let mut history = envelope.history;
    if history.parameters.len() > MAX_UPDATE_ASSETS || history.caps.len() > MAX_UPDATE_ASSETS {
        return Err("Updates history exceeds asset limit".into());
    }
    history
        .events
        .sort_by_key(|event| Reverse(event.detected_at_ms));
    history.events.truncate(MAX_UPDATE_EVENTS);
    Ok(history)
}

pub(crate) async fn save(history: UpdatesHistory) -> Result<(), String> {
    let Some(root) = crate::config::api_cache_dir() else {
        return Ok(());
    };
    tokio::task::spawn_blocking(move || save_to(&root.join("updates.json"), history))
        .await
        .map_err(|_| "Updates history save interrupted".to_string())?
}

fn save_to(path: &Path, history: UpdatesHistory) -> Result<(), String> {
    let bytes = serde_json::to_vec(&Envelope {
        version: 1,
        history,
    })
    .map_err(|_| "Updates history could not be encoded".to_string())?;
    crate::api_cache::write_bytes_atomic(path, &bytes)
        .map_err(|_| "Updates history could not be saved".to_string())
}

#[cfg(test)]
mod tests;
