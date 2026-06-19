mod task;

#[cfg(test)]
mod tests;

use super::HydromancerStreamKey;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use zeroize::Zeroizing;

const HYDROMANCER_READ_TIMEOUT_SECS: u64 = 95;
const HYDROMANCER_CONNECT_TIMEOUT_SECS: u64 = 10;
const HYDROMANCER_MAX_CONNECT_RETRY_SECS: u64 = 30;
const HYDROMANCER_IDLE_SHUTDOWN_SECS: u64 = 30;

/// Remaining time before forcing a hydromancer reconnect because no inbound
/// frames have arrived. Anchored to an absolute `last_rx_at` instant so the
/// deadline cannot be silently reset by command traffic landing on the
/// manager's `select` — that reset was the root cause of #13.
pub(super) fn hydromancer_read_remaining(last_rx_elapsed: Duration) -> Duration {
    Duration::from_secs(HYDROMANCER_READ_TIMEOUT_SECS).saturating_sub(last_rx_elapsed)
}

#[derive(Clone)]
pub(super) struct HydromancerRoutedMessage {
    pub(super) msg_type: String,
    pub(super) data: Arc<Value>,
}

impl fmt::Debug for HydromancerRoutedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HydromancerRoutedMessage")
            .field("msg_type", &self.msg_type)
            .field("data", &"<redacted>")
            .finish()
    }
}

pub(super) enum HydromancerCommand {
    Subscribe {
        topic: String,
        payload: Value,
    },
    Unsubscribe {
        topic: String,
        payload: Value,
    },
    Reconnect,
    /// Tear down the manager task entirely. Sent during API-key rotation
    /// so the previous key's task exits, dropping its owned `api_key`
    /// String instead of waiting indefinitely on `cmd_rx.recv()`.
    Shutdown,
}

impl fmt::Debug for HydromancerCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Subscribe { topic, payload } => f
                .debug_struct("Subscribe")
                .field("topic", &redacted_hydromancer_topic_debug_value(topic))
                .field("payload", &redacted_hydromancer_value(payload))
                .finish(),
            Self::Unsubscribe { topic, payload } => f
                .debug_struct("Unsubscribe")
                .field("topic", &redacted_hydromancer_topic_debug_value(topic))
                .field("payload", &redacted_hydromancer_value(payload))
                .finish(),
            Self::Reconnect => f.write_str("Reconnect"),
            Self::Shutdown => f.write_str("Shutdown"),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RedactedHydromancerValue<'a>(&'a Value);

impl fmt::Debug for RedactedHydromancerValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message_type = self.0.get("type").and_then(Value::as_str);
        let subscription_type = self
            .0
            .pointer("/subscription/type")
            .and_then(Value::as_str)
            .or(message_type);

        f.debug_struct("HydromancerJson")
            .field("type", &message_type)
            .field("subscription_type", &subscription_type)
            .field("payload", &"<redacted>")
            .finish()
    }
}

pub(super) fn redacted_hydromancer_value(value: &Value) -> RedactedHydromancerValue<'_> {
    RedactedHydromancerValue(value)
}

pub(super) fn redacted_hydromancer_topic_debug_value(topic: &str) -> &str {
    let lower = topic.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.contains(":0x") {
        "<redacted>"
    } else {
        topic
    }
}

struct HydromancerManager {
    task_id: u64,
    cmd_tx: HydromancerCommandSender,
    msg_rx: broadcast::Receiver<HydromancerRoutedMessage>,
}

static HYDROMANCER_MANAGERS: OnceLock<std::sync::Mutex<HashMap<u64, HydromancerManager>>> =
    OnceLock::new();
static NEXT_HYDROMANCER_MANAGER_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub(super) struct HydromancerReconnectGate(Arc<AtomicBool>);

impl HydromancerReconnectGate {
    fn request_lag_reconnect(&self, cmd_tx: &mpsc::UnboundedSender<HydromancerCommand>) -> bool {
        if self.0.swap(true, Ordering::AcqRel) {
            return true;
        }
        if cmd_tx.send(HydromancerCommand::Reconnect).is_err() {
            self.0.store(false, Ordering::Release);
            return false;
        }
        true
    }

    pub(super) fn note_dequeued(&self, command: &HydromancerCommand) {
        if matches!(command, HydromancerCommand::Reconnect) {
            self.0.store(false, Ordering::Release);
        }
    }
}

#[derive(Clone)]
pub(super) struct HydromancerCommandSender {
    inner: mpsc::UnboundedSender<HydromancerCommand>,
    reconnect_gate: HydromancerReconnectGate,
}

impl HydromancerCommandSender {
    fn new(
        inner: mpsc::UnboundedSender<HydromancerCommand>,
        reconnect_gate: HydromancerReconnectGate,
    ) -> Self {
        Self {
            inner,
            reconnect_gate,
        }
    }

    pub(super) fn send(
        &self,
        command: HydromancerCommand,
    ) -> Result<(), mpsc::error::SendError<HydromancerCommand>> {
        self.inner.send(command)
    }

    pub(super) fn request_reconnect(&self) -> bool {
        self.reconnect_gate.request_lag_reconnect(&self.inner)
    }

    pub(super) fn request_lag_reconnect(&self) -> bool {
        self.request_reconnect()
    }

    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    #[cfg(test)]
    pub(super) fn new_for_test(inner: mpsc::UnboundedSender<HydromancerCommand>) -> Self {
        Self::new(inner, HydromancerReconnectGate::default())
    }

    #[cfg(test)]
    pub(super) fn note_command_dequeued_for_test(&self, command: &HydromancerCommand) {
        self.reconnect_gate.note_dequeued(command);
    }
}

pub(super) struct HydromancerSubscriptionGuard {
    cmd_tx: HydromancerCommandSender,
    subscriptions: Vec<(String, Value)>,
}

impl HydromancerSubscriptionGuard {
    pub(super) fn new(
        cmd_tx: HydromancerCommandSender,
        subscriptions: Vec<(String, Value)>,
    ) -> Self {
        Self {
            cmd_tx,
            subscriptions,
        }
    }
}

impl Drop for HydromancerSubscriptionGuard {
    fn drop(&mut self) {
        for (topic, payload) in &self.subscriptions {
            let _ = self.cmd_tx.send(HydromancerCommand::Unsubscribe {
                topic: topic.clone(),
                payload: payload.clone(),
            });
        }
    }
}

fn next_hydromancer_manager_task_id() -> u64 {
    NEXT_HYDROMANCER_MANAGER_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

fn spawn_hydromancer_manager(manager_id: u64, api_key: Zeroizing<String>) -> HydromancerManager {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, msg_rx) = broadcast::channel(10000);
    let task_id = next_hydromancer_manager_task_id();
    let reconnect_gate = HydromancerReconnectGate::default();
    let command_sender = HydromancerCommandSender::new(cmd_tx, reconnect_gate.clone());
    tokio::spawn(async move {
        task::hydromancer_manager_task_with_reconnect_gate(api_key, cmd_rx, msg_tx, reconnect_gate)
            .await;
        remove_hydromancer_manager_if_finished(manager_id, task_id);
    });
    HydromancerManager {
        task_id,
        cmd_tx: command_sender,
        msg_rx,
    }
}

fn remove_hydromancer_manager_if_finished(manager_id: u64, task_id: u64) -> bool {
    let Some(managers) = HYDROMANCER_MANAGERS.get() else {
        return false;
    };
    let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    if managers
        .get(&manager_id)
        .is_some_and(|manager| manager.task_id == task_id && manager.cmd_tx.is_closed())
    {
        managers.remove(&manager_id);
        return true;
    }
    false
}

pub(super) fn get_hydromancer_manager(
    stream_key: HydromancerStreamKey,
) -> (
    HydromancerCommandSender,
    broadcast::Receiver<HydromancerRoutedMessage>,
) {
    let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    let manager_key = stream_key.manager_id();
    let api_key = stream_key.api_key_for_task();

    let manager = match managers.entry(manager_key) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            if entry.get().cmd_tx.is_closed() {
                entry.insert(spawn_hydromancer_manager(manager_key, api_key));
            }
            entry.into_mut()
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(spawn_hydromancer_manager(manager_key, api_key))
        }
    };

    (manager.cmd_tx.clone(), manager.msg_rx.resubscribe())
}

pub fn reconnect_hydromancer(stream_key: HydromancerStreamKey) {
    let Some(managers) = HYDROMANCER_MANAGERS.get() else {
        return;
    };
    let Ok(mut managers) = managers.lock() else {
        return;
    };
    let manager_id = stream_key.manager_id();
    if let Some(manager) = managers.get(&manager_id)
        && !manager.cmd_tx.request_reconnect()
    {
        managers.remove(&manager_id);
    }
}

#[cfg(test)]
pub(crate) fn hydromancer_manager_reconnect_sent_for_test(
    stream_key: HydromancerStreamKey,
    action: impl FnOnce(),
) -> bool {
    let manager_id = stream_key.manager_id();
    let managers = HYDROMANCER_MANAGERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let cmd_tx = HydromancerCommandSender::new_for_test(cmd_tx);
    let (_msg_tx, msg_rx) = broadcast::channel(1);
    {
        let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
        managers.remove(&manager_id);
        managers.insert(
            manager_id,
            HydromancerManager {
                task_id: next_hydromancer_manager_task_id(),
                cmd_tx,
                msg_rx,
            },
        );
    }

    action();

    let mut sent_reconnect = false;
    while let Ok(cmd) = cmd_rx.try_recv() {
        if matches!(cmd, HydromancerCommand::Reconnect) {
            sent_reconnect = true;
        }
    }

    let mut managers = managers.lock().unwrap_or_else(|e| e.into_inner());
    managers.remove(&manager_id);
    sent_reconnect
}

/// Tear down the Hydromancer manager for `stream_key` if one exists. Sends
/// `Shutdown` to the task (so its owned `api_key` String drops) and removes the
/// registry entry.
///
/// Intended for API-key rotation / clearing flows — every consumer that
/// re-subscribes after rotation will pick up the new key through
/// `get_hydromancer_manager`, which spawns a fresh task.
pub fn evict_hydromancer_manager(stream_key: HydromancerStreamKey) {
    let Some(managers) = HYDROMANCER_MANAGERS.get() else {
        return;
    };
    let Ok(mut managers) = managers.lock() else {
        return;
    };
    let manager_id = stream_key.manager_id();
    if let Some((_key, manager)) = managers.remove_entry(&manager_id) {
        // Best-effort shutdown signal. If the channel is already closed
        // the task is gone anyway.
        let _ = manager.cmd_tx.send(HydromancerCommand::Shutdown);
    }
}
