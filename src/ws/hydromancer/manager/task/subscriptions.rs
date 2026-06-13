use serde_json::Value;

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
    entries: Vec<ActiveHydromancerSubscription>,
}

#[derive(Debug)]
struct ActiveHydromancerSubscription {
    topic: String,
    count: usize,
    payload: Value,
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
