use super::super::HydromancerRoutedMessage;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Hydromancer Broadcast Coalescer
// ---------------------------------------------------------------------------

/// Keep Hydromancer book updates aligned with the exchange WS manager: the
/// latest `l2Book` snapshot per coin is enough for a paint frame, while other
/// Hydromancer channels remain unpaced.
pub(super) const HYDROMANCER_BOOK_COALESCE_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CoalesceKey {
    msg_type: String,
    coin: String,
}

#[derive(Debug)]
struct PendingEntry {
    deadline: Instant,
    message: HydromancerRoutedMessage,
}

pub(super) struct HydromancerCoalescedSender {
    inner: broadcast::Sender<HydromancerRoutedMessage>,
    last_emitted: HashMap<CoalesceKey, Instant>,
    pending: HashMap<CoalesceKey, PendingEntry>,
    interval: Duration,
}

impl HydromancerCoalescedSender {
    pub(super) fn new(inner: broadcast::Sender<HydromancerRoutedMessage>) -> Self {
        Self::with_interval(inner, HYDROMANCER_BOOK_COALESCE_INTERVAL)
    }

    fn with_interval(
        inner: broadcast::Sender<HydromancerRoutedMessage>,
        interval: Duration,
    ) -> Self {
        Self {
            inner,
            last_emitted: HashMap::new(),
            pending: HashMap::new(),
            interval,
        }
    }

    pub(super) fn submit_json(&mut self, json: Value) {
        let msg_type = hydromancer_msg_type(&json);
        self.submit(HydromancerRoutedMessage {
            msg_type,
            data: Arc::new(json),
        });
    }

    pub(super) fn submit(&mut self, message: HydromancerRoutedMessage) {
        if let Some(messages) = split_l2_book_batch_message(&message) {
            for message in messages {
                self.submit(message);
            }
            return;
        }

        let Some(key) = coalesce_key(&message) else {
            let _ = self.inner.send(message);
            return;
        };
        let now = Instant::now();

        match self.last_emitted.get(&key).copied() {
            Some(last) if now.duration_since(last) < self.interval => {
                let deadline = last + self.interval;
                self.pending.insert(key, PendingEntry { deadline, message });
            }
            _ => {
                let _ = self.inner.send(message);
                self.last_emitted.insert(key, now);
            }
        }
    }

    pub(super) fn next_due(&self) -> Option<Duration> {
        let now = Instant::now();
        self.pending
            .values()
            .map(|entry| entry.deadline.saturating_duration_since(now))
            .min()
    }

    pub(super) fn flush_due(&mut self) -> usize {
        let now = Instant::now();
        let due: Vec<CoalesceKey> = self
            .pending
            .iter()
            .filter(|(_, entry)| entry.deadline <= now)
            .map(|(key, _)| key.clone())
            .collect();
        let count = due.len();
        for key in due {
            if let Some(entry) = self.pending.remove(&key) {
                let _ = self.inner.send(entry.message);
                self.last_emitted.insert(key, now);
            }
        }
        count
    }

    pub(super) fn flush_all(&mut self) -> usize {
        let now = Instant::now();
        let pending = std::mem::take(&mut self.pending);
        let count = pending.len();
        for (key, entry) in pending {
            let _ = self.inner.send(entry.message);
            self.last_emitted.insert(key, now);
        }
        count
    }
}

fn hydromancer_msg_type(json: &Value) -> String {
    json.get("type")
        .or_else(|| json.get("channel"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn split_l2_book_batch_message(
    message: &HydromancerRoutedMessage,
) -> Option<Vec<HydromancerRoutedMessage>> {
    if message.msg_type != "l2Book" {
        return None;
    }

    let items = l2_book_batch_items(message.data.as_ref())?;
    if items.len() <= 1
        || !items
            .iter()
            .all(|item| item.get("coin").and_then(Value::as_str).is_some())
    {
        return None;
    }

    Some(
        items
            .into_iter()
            .map(|item| HydromancerRoutedMessage {
                msg_type: message.msg_type.clone(),
                data: Arc::new(item),
            })
            .collect(),
    )
}

fn l2_book_batch_items(value: &Value) -> Option<Vec<Value>> {
    value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.get("books").and_then(Value::as_array))
        .map(|items| items.to_vec())
}

fn coalesce_key(message: &HydromancerRoutedMessage) -> Option<CoalesceKey> {
    if message.msg_type != "l2Book" {
        return None;
    }

    single_l2_book_coin(message.data.as_ref()).map(|coin| CoalesceKey {
        msg_type: message.msg_type.clone(),
        coin,
    })
}

fn single_l2_book_coin(value: &Value) -> Option<String> {
    if let Some(coin) = value.get("coin").and_then(Value::as_str) {
        return Some(coin.to_string());
    }

    if let Some(data) = value.get("data") {
        if let Some(coin) = data.get("coin").and_then(Value::as_str) {
            return Some(coin.to_string());
        }
        if let Some(items) = data.as_array() {
            return single_coin_from_items(items.iter());
        }
    }

    value
        .get("books")
        .and_then(Value::as_array)
        .and_then(|items| single_coin_from_items(items.iter()))
}

fn single_coin_from_items<'a>(items: impl Iterator<Item = &'a Value>) -> Option<String> {
    let mut found: Option<&str> = None;
    for item in items {
        let coin = item.get("coin").and_then(Value::as_str)?;
        if found.is_some_and(|existing| existing != coin) {
            return None;
        }
        found = Some(coin);
    }
    found.map(str::to_string)
}
