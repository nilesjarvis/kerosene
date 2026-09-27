use super::WsRoutedMessage;
use crate::ws::coalescer::SnapshotCoalescer;
use crate::ws::{L2BookSigfigs, l2_book_sigfigs_from_value};

use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
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
/// for different coins and attributed book precisions on the same channel
/// don't smash each other's pending slots.
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
    sigfigs: L2BookSigfigs,
}

pub(super) struct CoalescedSender {
    snapshots: SnapshotCoalescer<CoalesceKey, WsRoutedMessage>,
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
            snapshots: SnapshotCoalescer::new(inner, interval),
        }
    }

    pub(super) fn submit(&mut self, channel: String, data: Arc<Value>) {
        let key = is_coalesced_channel(&channel).then(|| CoalesceKey {
            channel: channel.clone(),
            coin: extract_coin(&channel, &data),
            sigfigs: l2_book_sigfigs_from_value(&data),
        });
        self.snapshots
            .submit(key, WsRoutedMessage { channel, data });
    }

    pub(super) fn next_due(&self) -> Option<Duration> {
        self.snapshots.next_due()
    }

    pub(super) fn flush_due(&mut self) -> usize {
        self.snapshots.flush_due()
    }

    pub(super) fn flush_all(&mut self) -> usize {
        self.snapshots.flush_all()
    }
}
