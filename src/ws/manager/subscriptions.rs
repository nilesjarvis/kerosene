use super::{redacted_ws_topic_debug_value, redacted_ws_value};

use serde_json::Value;
use std::fmt;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Active Subscription Reference Counts
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(super) enum WsUnsubscribeResult {
    Missing,
    StillActive,
    Removed { unsubscribe_payload: Value },
}

impl fmt::Debug for WsUnsubscribeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("Missing"),
            Self::StillActive => f.write_str("StillActive"),
            Self::Removed {
                unsubscribe_payload,
            } => f
                .debug_struct("Removed")
                .field(
                    "unsubscribe_payload",
                    &redacted_ws_value(unsubscribe_payload),
                )
                .finish(),
        }
    }
}

impl WsUnsubscribeResult {
    pub(super) fn removed_payload(self) -> Option<Value> {
        match self {
            WsUnsubscribeResult::Removed {
                unsubscribe_payload,
            } => Some(unsubscribe_payload),
            WsUnsubscribeResult::Missing | WsUnsubscribeResult::StillActive => None,
        }
    }
}

#[derive(Default)]
pub(super) struct ActiveWsSubscriptions {
    entries: Vec<ActiveWsSubscription>,
}

struct ActiveWsSubscription {
    topic: String,
    count: usize,
    payload: Value,
}

impl fmt::Debug for ActiveWsSubscriptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveWsSubscriptions")
            .field("entries", &self.entries)
            .finish()
    }
}

impl fmt::Debug for ActiveWsSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveWsSubscription")
            .field("topic", &redacted_ws_topic_debug_value(&self.topic))
            .field("count", &self.count)
            .field("payload", &redacted_ws_value(&self.payload))
            .finish()
    }
}

impl ActiveWsSubscriptions {
    pub(super) fn subscribe(&mut self, topic: String, payload: Value) -> Option<Value> {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.topic == topic && entry.payload == payload)
        {
            entry.count += 1;
            return None;
        }

        let outbound_payload = payload.clone();
        self.entries.push(ActiveWsSubscription {
            topic,
            count: 1,
            payload,
        });
        Some(outbound_payload)
    }

    pub(super) fn unsubscribe(&mut self, topic: String, payload: Value) -> WsUnsubscribeResult {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.topic == topic && entry.payload == payload)
        else {
            return WsUnsubscribeResult::Missing;
        };

        let entry = &mut self.entries[index];
        entry.count = entry.count.saturating_sub(1);
        if entry.count > 0 {
            return WsUnsubscribeResult::StillActive;
        }

        let mut unsubscribe_payload = entry.payload.clone();
        if let Some(obj) = unsubscribe_payload.as_object_mut() {
            obj.insert("method".to_string(), serde_json::json!("unsubscribe"));
        }
        self.entries.remove(index);
        WsUnsubscribeResult::Removed {
            unsubscribe_payload,
        }
    }

    pub(super) fn payloads(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|entry| &entry.payload)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
