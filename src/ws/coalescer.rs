use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

struct PendingEntry<T> {
    deadline: Instant,
    message: T,
}

/// Pace replaceable snapshots per key while passing unkeyed messages through.
/// Providers decide which frames are snapshots and what distinguishes a key.
pub(super) struct SnapshotCoalescer<K, T> {
    inner: broadcast::Sender<T>,
    last_emitted: HashMap<K, Instant>,
    pending: HashMap<K, PendingEntry<T>>,
    interval: Duration,
}

impl<K: Clone + Eq + Hash, T: Clone> SnapshotCoalescer<K, T> {
    pub(super) fn new(inner: broadcast::Sender<T>, interval: Duration) -> Self {
        Self {
            inner,
            last_emitted: HashMap::new(),
            pending: HashMap::new(),
            interval,
        }
    }

    pub(super) fn submit(&mut self, key: Option<K>, message: T) {
        let Some(key) = key else {
            let _ = self.inner.send(message);
            return;
        };
        let now = Instant::now();
        self.prune_stale_last_emitted(now);

        match self.last_emitted.get(&key).copied() {
            Some(last) if now.duration_since(last) < self.interval => {
                let deadline = last + self.interval;
                self.pending.insert(key, PendingEntry { deadline, message });
            }
            _ => {
                self.pending.remove(&key);
                let _ = self.inner.send(message);
                self.last_emitted.insert(key, now);
            }
        }
    }

    fn prune_stale_last_emitted(&mut self, now: Instant) {
        let interval = self.interval;
        let pending = &self.pending;
        self.last_emitted
            .retain(|key, last| pending.contains_key(key) || now.duration_since(*last) < interval);
    }

    pub(super) fn next_due(&self) -> Option<Duration> {
        let now = Instant::now();
        self.pending
            .values()
            .map(|entry| entry.deadline.saturating_duration_since(now))
            .min()
    }

    /// Emit expired entries and retain later deadlines. The most recent
    /// payload wins without extending a pending entry's original deadline.
    pub(super) fn flush_due(&mut self) -> usize {
        let now = Instant::now();
        let due: Vec<K> = self
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

    /// Flush snapshots before a socket is dropped or replaced so a disconnect
    /// within the pacing interval does not lose the latest book.
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

    #[cfg(test)]
    pub(super) fn history_len(&self) -> usize {
        self.last_emitted.len()
    }
}

#[cfg(test)]
mod tests;
