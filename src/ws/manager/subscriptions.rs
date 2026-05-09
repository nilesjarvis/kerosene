use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Active Subscription Reference Counts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(super) enum WsUnsubscribeResult {
    Missing,
    StillActive,
    Removed { unsubscribe_payload: Value },
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

#[derive(Debug, Default)]
pub(super) struct ActiveWsSubscriptions {
    entries: HashMap<String, (usize, Value)>,
}

impl ActiveWsSubscriptions {
    pub(super) fn subscribe(&mut self, topic: String, payload: Value) -> Option<Value> {
        let entry = self.entries.entry(topic).or_insert((0, payload));
        entry.0 += 1;

        if entry.0 == 1 {
            Some(entry.1.clone())
        } else {
            None
        }
    }

    pub(super) fn unsubscribe(&mut self, topic: String) -> WsUnsubscribeResult {
        let Some((count, payload)) = self.entries.get_mut(&topic) else {
            return WsUnsubscribeResult::Missing;
        };

        *count = count.saturating_sub(1);
        if *count > 0 {
            return WsUnsubscribeResult::StillActive;
        }

        let mut unsubscribe_payload = payload.clone();
        if let Some(obj) = unsubscribe_payload.as_object_mut() {
            obj.insert("method".to_string(), serde_json::json!("unsubscribe"));
        }
        self.entries.remove(&topic);
        WsUnsubscribeResult::Removed {
            unsubscribe_payload,
        }
    }

    pub(super) fn payloads(&self) -> impl Iterator<Item = &Value> {
        self.entries.values().map(|(_, payload)| payload)
    }
}
