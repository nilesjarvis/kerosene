use crate::config::updates::{MAX_UPDATE_ASSETS, MAX_UPDATE_EVENTS, UpdatesHistory};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, io::Read, path::Path};

const MAX_UPDATES_FILE_BYTES: usize = 16 * 1024 * 1024;

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
    let envelope: Envelope = serde_json::from_reader(file.take(MAX_UPDATES_FILE_BYTES as u64))
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
    let bytes = encode(
        &Envelope {
            version: 1,
            history,
        },
        MAX_UPDATES_FILE_BYTES,
    )?;
    crate::api_cache::write_bytes_atomic(path, &bytes)
        .map_err(|_| "Updates history could not be saved".to_string())
}

fn encode(envelope: &Envelope, limit: usize) -> Result<Vec<u8>, String> {
    struct LimitedVec {
        bytes: Vec<u8>,
        limit: usize,
    }

    impl std::io::Write for LimitedVec {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.bytes.len().saturating_add(bytes.len()) > self.limit {
                return Err(std::io::Error::other("updates history exceeds size limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut output = LimitedVec {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, envelope)
        .map_err(|_| "Updates history exceeds persistence size limit".to_string())?;
    Ok(output.bytes)
}

#[cfg(test)]
mod tests;
