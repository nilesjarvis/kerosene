use crate::api::Candle;
use crate::config::ChartBackfillSource;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};

use super::candles::{
    CANDLE_CACHE_NAMESPACE, candle_key, merge_candle_page_into_dir, save_candle_snapshot_at,
};
use super::storage::{json_path, remove_json_file, write_bytes_atomic};

// ---------------------------------------------------------------------------
// Background cache writer
//
// Cache writes serialize candle vectors and `fsync` files. Candle rollovers,
// REST pages, and watchlist contexts can all enqueue writes from live update
// paths, so none of that may block the iced thread or a tokio worker. Every
// write is handed to one dedicated thread. Routing merges through that thread
// also serializes read-modify-write operations so concurrent fetches of the
// same key cannot clobber one another. Bursts are coalesced: a plain save made
// redundant by a later save/removal of the same file is dropped before I/O.
// ---------------------------------------------------------------------------

pub(super) enum CacheWrite {
    SaveCandles {
        root: PathBuf,
        source: ChartBackfillSource,
        symbol: String,
        interval: String,
        candles: Vec<Candle>,
        observed_at_ms: u64,
    },
    MergeCandles {
        root: PathBuf,
        source: ChartBackfillSource,
        symbol: String,
        interval: String,
        candles: Vec<Candle>,
        observed_at_ms: u64,
    },
    SaveBytes {
        path: PathBuf,
        bytes: Vec<u8>,
    },
    Remove {
        path: PathBuf,
    },
}

impl CacheWrite {
    /// The cache file this job targets, used to coalesce bursts to one key.
    fn target(&self) -> PathBuf {
        match self {
            CacheWrite::SaveCandles {
                root,
                source,
                symbol,
                interval,
                ..
            }
            | CacheWrite::MergeCandles {
                root,
                source,
                symbol,
                interval,
                ..
            } => json_path(
                root,
                CANDLE_CACHE_NAMESPACE,
                &candle_key(*source, symbol, interval),
            ),
            CacheWrite::SaveBytes { path, .. } | CacheWrite::Remove { path } => path.clone(),
        }
    }

    fn is_plain_save(&self) -> bool {
        matches!(
            self,
            CacheWrite::SaveCandles { .. } | CacheWrite::SaveBytes { .. }
        )
    }

    /// Whether this job makes an earlier plain save to the same target
    /// redundant. A later merge does NOT (it reads the saved file first).
    fn supersedes_save(&self) -> bool {
        matches!(
            self,
            CacheWrite::SaveCandles { .. }
                | CacheWrite::SaveBytes { .. }
                | CacheWrite::Remove { .. }
        )
    }

    fn run(self) {
        match self {
            CacheWrite::SaveCandles {
                root,
                source,
                symbol,
                interval,
                candles,
                observed_at_ms,
            } => {
                let _ = save_candle_snapshot_at(
                    root,
                    source,
                    &symbol,
                    &interval,
                    candles,
                    observed_at_ms,
                );
            }
            CacheWrite::MergeCandles {
                root,
                source,
                symbol,
                interval,
                candles,
                observed_at_ms,
            } => {
                let _ = merge_candle_page_into_dir(
                    &root,
                    source,
                    &symbol,
                    &interval,
                    candles,
                    observed_at_ms,
                );
            }
            CacheWrite::SaveBytes { path, bytes } => {
                let _ = write_bytes_atomic(&path, &bytes);
            }
            CacheWrite::Remove { path } => {
                let _ = remove_json_file(path);
            }
        }
    }
}

/// For each job in a drained batch, whether it should actually run. A plain save
/// is dropped when a later job for the same target supersedes it.
fn writes_to_run(batch: &[CacheWrite]) -> Vec<bool> {
    let mut later_superseders = HashSet::new();
    let mut run = vec![true; batch.len()];
    for (index, job) in batch.iter().enumerate().rev() {
        if job.supersedes_save() {
            let is_latest = later_superseders.insert(job.target());
            run[index] = !job.is_plain_save() || is_latest;
        }
    }
    run
}

fn run_write_batch(batch: Vec<CacheWrite>) {
    let mask = writes_to_run(&batch);
    for (run, job) in mask.into_iter().zip(batch) {
        if run {
            job.run();
        }
    }
}

enum CacheWriter {
    Background(Sender<CacheWrite>),
    /// Spawning the writer thread failed; fall back to writing inline.
    Inline,
}

fn cache_writer() -> &'static CacheWriter {
    static WRITER: OnceLock<CacheWriter> = OnceLock::new();
    WRITER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<CacheWrite>();
        let spawned = std::thread::Builder::new()
            .name("kerosene-cache-writer".to_string())
            .spawn(move || {
                // `recv` only errors once every sender is dropped; the sender is
                // held in a static, so this loop lives for the process lifetime.
                while let Ok(first) = rx.recv() {
                    let mut batch = vec![first];
                    while let Ok(next) = rx.try_recv() {
                        batch.push(next);
                    }
                    run_write_batch(batch);
                }
            });
        match spawned {
            Ok(_) => CacheWriter::Background(tx),
            Err(_) => CacheWriter::Inline,
        }
    })
}

pub(super) fn enqueue(job: CacheWrite) {
    match cache_writer() {
        CacheWriter::Background(tx) => {
            let _ = tx.send(job);
        }
        CacheWriter::Inline => job.run(),
    }
}

#[cfg(test)]
mod tests;
