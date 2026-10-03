use super::super::super::capacity::{MARKET_FALLBACK_MESSAGE, has_market_fallback};
use super::super::{
    HydromancerRoutedMessage, redacted_hydromancer_topic_debug_value, redacted_hydromancer_value,
};
use super::messages::broadcast_hydromancer_control;

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
    market_fallback: bool,
    msg_tx: Option<tokio::sync::broadcast::Sender<HydromancerRoutedMessage>>,
}

struct ActiveHydromancerSubscription {
    topic: String,
    count: usize,
    repaired_at: Option<std::time::Instant>,
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
    pub(super) fn new(msg_tx: tokio::sync::broadcast::Sender<HydromancerRoutedMessage>) -> Self {
        Self {
            msg_tx: Some(msg_tx),
            ..Self::default()
        }
    }

    pub(super) fn notify_market_fallback(&self) {
        if self.market_fallback
            && let Some(msg_tx) = &self.msg_tx
        {
            let _ = broadcast_hydromancer_control(
                msg_tx,
                MARKET_FALLBACK_MESSAGE,
                serde_json::json!({}),
            );
        }
    }

    /// Latch fallback for this key's manager, including consumers added later.
    /// Removing desired subscriptions also prevents reconnect/watchdog replay
    /// from reclaiming capacity before the consumers drop their guards.
    pub(super) fn fallback_market_streams(&mut self) -> Vec<Value> {
        self.market_fallback = true;
        let mut retired = Vec::new();
        self.entries.retain(|entry| {
            if has_market_fallback(&entry.payload) {
                retired.push(entry.payload.clone());
                false
            } else {
                true
            }
        });
        self.notify_market_fallback();
        retired
    }

    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn subscribe(&mut self, topic: String, payload: Value) -> Option<Value> {
        if self.market_fallback && has_market_fallback(&payload) {
            self.notify_market_fallback();
            return None;
        }
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
            repaired_at: None,
            payload,
        });
        // Replay scarce-provider feeds before replaceable market subscriptions,
        // regardless of the order in which iced started their consumers.
        self.entries.sort_by_key(|entry| {
            match entry
                .payload
                .pointer("/subscription/type")
                .and_then(Value::as_str)
            {
                Some("liquidationFills") => 0,
                Some("userFills") => 1,
                _ if !has_market_fallback(&entry.payload) => 2,
                _ => 3,
            }
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

        let payload = self.entries.remove(index).payload;
        HydromancerUnsubscribeResult::Removed {
            payload,
            became_empty: self.entries.is_empty(),
        }
    }

    pub(super) fn resubscribe(&mut self, topic: &str) -> Option<Value> {
        // A slow consumer may have lost the original broadcast notice. Its
        // watchdog must route it to fallback instead of reviving a retired topic.
        self.notify_market_fallback();
        let entry = self.entries.iter_mut().find(|entry| entry.topic == topic)?;
        let now = std::time::Instant::now();
        if entry
            .repaired_at
            .is_some_and(|at| now.duration_since(at) < std::time::Duration::from_secs(60))
        {
            return None;
        }
        entry.repaired_at = Some(now);
        Some(entry.payload.clone())
    }

    pub(super) fn payloads(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|entry| &entry.payload)
    }
}
