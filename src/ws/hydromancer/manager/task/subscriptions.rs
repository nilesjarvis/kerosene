use super::super::{redacted_hydromancer_topic_debug_value, redacted_hydromancer_value};

use serde_json::Value;
use std::fmt;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Active Subscription Reference Counts
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(super) enum HydromancerUnsubscribeResult {
    Missing,
    StillActive,
    Removed { payload: Value, became_empty: bool },
}

impl fmt::Debug for HydromancerUnsubscribeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("Missing"),
            Self::StillActive => f.write_str("StillActive"),
            Self::Removed {
                payload,
                became_empty,
            } => f
                .debug_struct("Removed")
                .field("payload", &redacted_hydromancer_value(payload))
                .field("became_empty", became_empty)
                .finish(),
        }
    }
}

#[derive(Default)]
pub(super) struct ActiveHydromancerSubscriptions {
    entries: Vec<ActiveHydromancerSubscription>,
}

struct ActiveHydromancerSubscription {
    topic: String,
    count: usize,
    payload: Value,
}

impl fmt::Debug for ActiveHydromancerSubscriptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveHydromancerSubscriptions")
            .field("entries", &self.entries)
            .finish()
    }
}

impl fmt::Debug for ActiveHydromancerSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveHydromancerSubscription")
            .field(
                "topic",
                &redacted_hydromancer_topic_debug_value(&self.topic),
            )
            .field("count", &self.count)
            .field("payload", &redacted_hydromancer_value(&self.payload))
            .finish()
    }
}

impl ActiveHydromancerSubscriptions {
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

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
        self.entries.push(ActiveHydromancerSubscription {
            topic,
            count: 1,
            payload,
        });
        Some(outbound_payload)
    }

    pub(super) fn unsubscribe(
        &mut self,
        topic: String,
        payload: Value,
    ) -> HydromancerUnsubscribeResult {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.topic == topic && entry.payload == payload)
        else {
            return HydromancerUnsubscribeResult::Missing;
        };

        let entry = &mut self.entries[index];
        entry.count = entry.count.saturating_sub(1);
        if entry.count > 0 {
            return HydromancerUnsubscribeResult::StillActive;
        }

        let payload = entry.payload.clone();
        self.entries.remove(index);
        HydromancerUnsubscribeResult::Removed {
            payload,
            became_empty: self.entries.is_empty(),
        }
    }

    pub(super) fn payloads(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|entry| &entry.payload)
    }
}
