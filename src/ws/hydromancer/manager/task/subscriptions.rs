use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Active Subscription Reference Counts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(super) enum HydromancerUnsubscribeResult {
    Missing,
    StillActive,
    Removed { payload: Value, became_empty: bool },
}

#[derive(Debug, Default)]
pub(super) struct ActiveHydromancerSubscriptions {
    entries: HashMap<String, (usize, Value)>,
}

impl ActiveHydromancerSubscriptions {
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn subscribe(&mut self, topic: String, payload: Value) -> Option<Value> {
        let entry = self.entries.entry(topic).or_insert((0, payload));
        entry.0 += 1;

        if entry.0 == 1 {
            Some(entry.1.clone())
        } else {
            None
        }
    }

    pub(super) fn unsubscribe(&mut self, topic: String) -> HydromancerUnsubscribeResult {
        let Some((count, payload)) = self.entries.get_mut(&topic) else {
            return HydromancerUnsubscribeResult::Missing;
        };

        *count = count.saturating_sub(1);
        if *count > 0 {
            return HydromancerUnsubscribeResult::StillActive;
        }

        let payload = payload.clone();
        self.entries.remove(&topic);
        HydromancerUnsubscribeResult::Removed {
            payload,
            became_empty: self.entries.is_empty(),
        }
    }

    pub(super) fn payloads(&self) -> impl Iterator<Item = &Value> {
        self.entries.values().map(|(_, payload)| payload)
    }
}
