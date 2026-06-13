use serde_json::Value;

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
    entries: Vec<ActiveWsSubscription>,
}

#[derive(Debug)]
struct ActiveWsSubscription {
    topic: String,
    count: usize,
    payload: Value,
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
