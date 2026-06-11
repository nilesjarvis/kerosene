mod task;

#[cfg(test)]
mod tests;

use serde_json::Value;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use zeroize::Zeroize;

use self::task::hydromancer_manager_task;

const HYDROMANCER_READ_TIMEOUT_SECS: u64 = 95;
const HYDROMANCER_MAX_CONNECT_RETRY_SECS: u64 = 30;

/// Remaining time before forcing a hydromancer reconnect because no inbound
/// frames have arrived. Anchored to an absolute `last_rx_at` instant so the
/// deadline cannot be silently reset by command traffic landing on the
/// manager's `select` — that reset was the root cause of #13.
pub(super) fn hydromancer_read_remaining(last_rx_elapsed: Duration) -> Duration {
    Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS).saturating_sub(last_rx_elapsed)
}

#[derive(Clone, Debug)]
pub(super) struct HydromancerRoutedMessage {
    pub(super) msg_type: String,
    pub(super) data: Arc<Value>,
}

#[derive(Debug)]
pub(super) enum HydromancerCommand {
    Subscribe {
        topic: String,
        payload: Value,
    },
    Unsubscribe {
        topic: String,
    },
    Reconnect,
    /// Tear down the manager task entirely. Sent during API-key rotation
    /// so the previous key's task exits, dropping its owned `api_key`
    /// String instead of waiting indefinitely on `cmd_rx.recv()`.
    Shutdown,
}

struct HydromancerManager {
    cmd_tx: mpsc::UnboundedSender<HydromancerCommand>,
    msg_rx: broadcast::Receiver<HydromancerRoutedMessage>,
}

static HYDROMANCER_MANAGERS: OnceLock<std::sync::Mutex<HashMap<String, HydromancerManager>>> =
    OnceLock::new();

fn hydromancer_manager_key(api_key: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(api_key.trim().as_bytes());
    hex::encode(hasher.finalize())
}

pub(super) struct HydromancerSubscriptionGuard {
    cmd_tx: mpsc::UnboundedSender<HydromancerCommand>,
    topics: Vec<String>,
}

impl HydromancerSubscriptionGuard {
    pub(super) fn new(
        cmd_tx: mpsc::UnboundedSender<HydromancerCommand>,
        topics: Vec<String>,
    ) -> Self {
        Self { cmd_tx, topics }
    }
}

impl Drop for HydromancerSubscriptionGuard {
    fn drop(&mut self) {
        for topic in &self.topics {
            let _ = self.cmd_tx.send(HydromancerCommand::Unsubscribe {
                topic: topic.clone(),
            });
        }
    }
}

fn spawn_hydromancer_manager(api_key: String) -> HydromancerManager {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, msg_rx) = broadcast::channel(10000);
    tokio::spawn(hydromancer_manager_task(api_key, cmd_rx, msg_tx));
    HydromancerManager { cmd_tx, msg_rx }
}

pub(super) fn get_hydromancer_manager(
    mut api_key: String,
) -> (
    mpsc::UnboundedSender<HydromancerCommand>,
    broadcast::Receiver<HydromancerRoutedMessage>,
) {
    let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    let manager_key = hydromancer_manager_key(&api_key);

    let manager = match managers.entry(manager_key) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            if entry.get().cmd_tx.is_closed() {
                entry.insert(spawn_hydromancer_manager(api_key));
            } else {
                api_key.zeroize();
            }
            entry.into_mut()
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(spawn_hydromancer_manager(api_key))
        }
    };

    (manager.cmd_tx.clone(), manager.msg_rx.resubscribe())
}

pub fn reconnect_hydromancer(api_key: &str) {
    let Some(managers) = HYDROMANCER_MANAGERS.get() else {
        return;
    };
    let Ok(managers) = managers.lock() else {
        return;
    };
    let manager_key = hydromancer_manager_key(api_key);
    if let Some(manager) = managers.get(&manager_key) {
        let _ = manager.cmd_tx.send(HydromancerCommand::Reconnect);
    }
}

/// Tear down the Hydromancer manager for `api_key` if one exists. Sends
/// `Shutdown` to the task (so its owned `api_key` String drops) and
/// removes the registry entry.
///
/// Intended for API-key rotation / clearing flows — every consumer that
/// re-subscribes after rotation will pick up the new key through
/// `get_hydromancer_manager`, which spawns a fresh task.
pub fn evict_hydromancer_manager(api_key: &str) {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return;
    }
    let Some(managers) = HYDROMANCER_MANAGERS.get() else {
        return;
    };
    let Ok(mut managers) = managers.lock() else {
        return;
    };
    let manager_key = hydromancer_manager_key(trimmed);
    if let Some((_key, manager)) = managers.remove_entry(&manager_key) {
        // Best-effort shutdown signal. If the channel is already closed
        // the task is gone anyway.
        let _ = manager.cmd_tx.send(HydromancerCommand::Shutdown);
    }
}
