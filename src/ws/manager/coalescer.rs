use super::WsRoutedMessage;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Broadcast Coalescer
//
// Hyperliquid pushes the L2 book as a fresh full snapshot many times per
// second; consecutive snapshots replace each other entirely. For subscribers
// that re-render or re-aggregate on every wake-up (chart cache invalidation,
// DOM ladder relayout) the intermediate snapshots between paints are wasted
// work — only the latest snapshot at paint time matters.
//
// This wrapper holds back updates for "coalesced" channels (currently
// `l2Book`) within a small window and emits only the latest per coin. Other
// channels (user fills, orders, trades, account updates) pass through
// untouched.
// ---------------------------------------------------------------------------

/// Maximum interval that book updates are held back before being forwarded
/// to subscribers. ~one 60fps frame; small enough that the DOM ladder still
/// feels live, large enough to absorb several snapshots per coin.
pub(super) const COALESCE_INTERVAL: Duration = Duration::from_millis(16);

/// Channels routed through the coalescer. Everything else passes through
/// without any state being recorded.
fn is_coalesced_channel(channel: &str) -> bool {
    matches!(channel, "l2Book")
}

/// Extract a per-stream discriminator from the frame body so that updates
/// for different coins on the same channel don't smash each other's pending
/// slots.
fn extract_coin(channel: &str, data: &Value) -> Option<String> {
    match channel {
        "l2Book" => data.get("coin").and_then(|v| v.as_str()).map(str::to_owned),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CoalesceKey {
    channel: String,
    coin: Option<String>,
}

#[derive(Debug)]
struct PendingEntry {
    deadline: Instant,
    data: Arc<Value>,
}

pub(super) struct CoalescedSender {
    inner: broadcast::Sender<WsRoutedMessage>,
    last_emitted: HashMap<CoalesceKey, Instant>,
    pending: HashMap<CoalesceKey, PendingEntry>,
    interval: Duration,
}

impl CoalescedSender {
    pub(super) fn new(inner: broadcast::Sender<WsRoutedMessage>) -> Self {
        Self::with_interval(inner, COALESCE_INTERVAL)
    }

    pub(super) fn with_interval(
        inner: broadcast::Sender<WsRoutedMessage>,
        interval: Duration,
    ) -> Self {
        Self {
            inner,
            last_emitted: HashMap::new(),
            pending: HashMap::new(),
            interval,
        }
    }

    /// Route a parsed frame. Coalesced channels are emitted at most once per
    /// `interval` per (channel, coin) key; the most recent payload wins.
    pub(super) fn submit(&mut self, channel: String, data: Arc<Value>) {
        if !is_coalesced_channel(&channel) {
            let _ = self.inner.send(WsRoutedMessage { channel, data });
            return;
        }

        let coin = extract_coin(&channel, &data);
        let key = CoalesceKey {
            channel: channel.clone(),
            coin,
        };
        let now = Instant::now();

        match self.last_emitted.get(&key).copied() {
            Some(last) if now.duration_since(last) < self.interval => {
                let deadline = last + self.interval;
                self.pending.insert(key, PendingEntry { deadline, data });
            }
            _ => {
                self.pending.remove(&key);
                let _ = self.inner.send(WsRoutedMessage { channel, data });
                self.last_emitted.insert(key, now);
            }
        }
    }

    /// Time until the next pending entry is due to flush, or `None` when
    /// nothing is queued.
    pub(super) fn next_due(&self) -> Option<Duration> {
        let now = Instant::now();
        self.pending
            .values()
            .map(|entry| entry.deadline.saturating_duration_since(now))
            .min()
    }

    /// Emit any pending entries whose deadline has elapsed. Returns the
    /// number of entries flushed (for tests and telemetry).
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
                let _ = self.inner.send(WsRoutedMessage {
                    channel: key.channel.clone(),
                    data: entry.data,
                });
                self.last_emitted.insert(key, now);
            }
        }
        count
    }

    /// Emit every queued entry before the manager drops or replaces the active
    /// socket. This prevents a disconnect inside the coalescing window from
    /// losing the latest book snapshot.
    pub(super) fn flush_all(&mut self) -> usize {
        let now = Instant::now();
        let pending = std::mem::take(&mut self.pending);
        let count = pending.len();
        for (key, entry) in pending {
            let _ = self.inner.send(WsRoutedMessage {
                channel: key.channel.clone(),
                data: entry.data,
            });
            self.last_emitted.insert(key, now);
        }
        count
    }
}
