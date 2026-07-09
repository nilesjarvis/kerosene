use crate::api::{
    Candle, ExchangeSymbolsPayload, WatchlistContext, candles_have_interior_gap, normalize_candles,
    trailing_contiguous_run_start,
};
use crate::config::{self, ChartBackfillSource};
use crate::timeframe::Timeframe;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const CACHE_SCHEMA_VERSION: u32 = 1;
const MAX_CACHED_CANDLES: usize = 12_000;
const EXCHANGE_SYMBOLS_FRESH_MS: u64 = 6 * 60 * 60 * 1000;
const WATCHLIST_CONTEXT_FRESH_MS: u64 = 15_000;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub(crate) struct CachedPayload<T> {
    pub(crate) fetched_at_ms: u64,
    pub(crate) complete_through_ms: Option<u64>,
    pub(crate) payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEnvelope<T> {
    schema_version: u32,
    namespace: String,
    key: Vec<String>,
    fetched_at_ms: u64,
    complete_through_ms: Option<u64>,
    payload: T,
}

pub(crate) fn load_fresh_candles(
    source: ChartBackfillSource,
    symbol: &str,
    timeframe: Timeframe,
    now_ms: u64,
) -> Result<Option<Vec<Candle>>, String> {
    load_fresh_candles_from_dir(&cache_root()?, source, symbol, timeframe, now_ms)
}

fn load_fresh_candles_from_dir(
    root: &Path,
    source: ChartBackfillSource,
    symbol: &str,
    timeframe: Timeframe,
    now_ms: u64,
) -> Result<Option<Vec<Candle>>, String> {
    let Some(candles) = load_candle_snapshot_from_dir(root, source, symbol, timeframe.api_str())?
    else {
        return Ok(None);
    };
    let Some(last_time) = candles.last().map(|candle| candle.open_time) else {
        return Ok(None);
    };
    if now_ms.saturating_sub(last_time) > timeframe.cache_display_max_age_ms() {
        return Ok(None);
    }
    // Never surface a snapshot across an interior gap: a stale block stitched to
    // a fresh tail (e.g. by a sleep/wake live append) would otherwise be served
    // and re-saved on every boot, rendering as a phantom price jump. Serve only
    // the trailing contiguous run; older history is repopulated by backfill.
    let mut candles = candles;
    let start = trailing_contiguous_run_start(&candles, timeframe.duration_ms());
    if start > 0 {
        candles.drain(0..start);
    }
    Ok((!candles.is_empty()).then_some(candles))
}

pub(crate) fn load_candles_for_range(
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    start_time: u64,
    end_time: u64,
) -> Result<Option<Vec<Candle>>, String> {
    load_candles_for_range_from_dir(
        &cache_root()?,
        source,
        symbol,
        interval,
        start_time,
        end_time,
    )
}

pub(crate) fn save_candles_snapshot(
    source: ChartBackfillSource,
    symbol: &str,
    timeframe: Timeframe,
    candles: Vec<Candle>,
) -> Result<(), String> {
    enqueue(CacheWrite::SaveCandles {
        root: cache_root()?,
        source,
        symbol: symbol.to_string(),
        interval: timeframe.api_str().to_string(),
        candles,
    });
    Ok(())
}

pub(crate) fn merge_candle_page(
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    candles: Vec<Candle>,
) -> Result<(), String> {
    enqueue(CacheWrite::MergeCandles {
        root: cache_root()?,
        source,
        symbol: symbol.to_string(),
        interval: interval.to_string(),
        candles,
    });
    Ok(())
}

pub(crate) fn remove_candles(
    source: ChartBackfillSource,
    symbol: &str,
    timeframe: Timeframe,
) -> Result<(), String> {
    let path = json_path(
        &cache_root()?,
        "candles",
        &candle_key(source, symbol, timeframe.api_str()),
    );
    enqueue(CacheWrite::Remove { path });
    Ok(())
}

/// Whether the on-disk candle cache may be consulted for this source/timeframe.
/// 1s Hydromancer candles can only be (re)fetched with an API key, so a cached
/// snapshot must not be surfaced when the key is absent.
pub(crate) fn cache_eligible(
    source: ChartBackfillSource,
    timeframe: Timeframe,
    hydromancer_api_key: &str,
) -> bool {
    !(source == ChartBackfillSource::Hydromancer
        && timeframe.requires_hydromancer_backfill()
        && hydromancer_api_key.trim().is_empty())
}

pub(crate) fn load_fresh_exchange_symbols(
    now_ms: u64,
) -> Result<Option<ExchangeSymbolsPayload>, String> {
    let Some(cached) = load_json::<ExchangeSymbolsPayload>(
        &cache_root()?,
        "market_metadata",
        &["exchange_symbols".to_string()],
    )?
    else {
        return Ok(None);
    };
    if now_ms.saturating_sub(cached.fetched_at_ms) > EXCHANGE_SYMBOLS_FRESH_MS {
        return Ok(None);
    }
    if !cached.payload.is_cacheable() {
        // Older cache entries did not retain the quote token for spot pairs;
        // partial or malformed payloads must likewise never suppress a fresh
        // metadata request in trading code.
        return Ok(None);
    }
    Ok(Some(cached.payload))
}

pub(crate) fn save_exchange_symbols(payload: &ExchangeSymbolsPayload) -> Result<(), String> {
    if !payload.is_cacheable() {
        return Err("refusing to cache incomplete exchange symbol metadata".to_string());
    }
    let (path, bytes) = envelope_bytes(
        &cache_root()?,
        "market_metadata",
        &["exchange_symbols".to_string()],
        now_ms(),
        None,
        payload,
    )?;
    enqueue(CacheWrite::SaveBytes { path, bytes });
    Ok(())
}

pub(crate) fn load_fresh_watchlist_contexts(
    symbols: &[String],
    now_ms: u64,
) -> Result<Option<HashMap<String, WatchlistContext>>, String> {
    let root = cache_root()?;
    load_fresh_watchlist_contexts_from_dir(&root, symbols, now_ms)
}

pub(crate) fn save_watchlist_contexts(
    contexts: &HashMap<String, WatchlistContext>,
) -> Result<(), String> {
    let root = cache_root()?;
    let fetched_at_ms = now_ms();
    for (symbol, context) in contexts {
        let (path, bytes) = envelope_bytes(
            &root,
            "watchlist_contexts",
            std::slice::from_ref(symbol),
            fetched_at_ms,
            None,
            context,
        )?;
        enqueue(CacheWrite::SaveBytes { path, bytes });
    }
    Ok(())
}

fn load_candle_snapshot_from_dir(
    root: &Path,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
) -> Result<Option<Vec<Candle>>, String> {
    let Some(cached) =
        load_json::<Vec<Candle>>(root, "candles", &candle_key(source, symbol, interval))?
    else {
        return Ok(None);
    };
    let candles = normalize_candles(cached.payload);
    Ok((!candles.is_empty()).then_some(candles))
}

fn load_candles_for_range_from_dir(
    root: &Path,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    start_time: u64,
    end_time: u64,
) -> Result<Option<Vec<Candle>>, String> {
    let Some(cached) =
        load_json::<Vec<Candle>>(root, "candles", &candle_key(source, symbol, interval))?
    else {
        return Ok(None);
    };
    let candles = normalize_candles(cached.payload);
    if candles.is_empty()
        || !candles_cover_range(
            &candles,
            interval,
            start_time,
            end_time,
            cached.complete_through_ms,
        )
    {
        return Ok(None);
    }

    let subset = candles
        .into_iter()
        .filter(|candle| candle.close_time >= start_time && candle.open_time <= end_time)
        .collect::<Vec<_>>();
    if subset.is_empty() {
        return Ok(None);
    }
    // `candles_cover_range` only checks the endpoints, so a cached snapshot with
    // an interior hole can still satisfy a spanning range. Miss in that case so
    // the caller fetches the gap from the network instead of serving a gapped
    // subset that would be merged straight into the chart.
    if let Some(interval_ms) = candle_interval_ms(interval)
        && candles_have_interior_gap(&subset, interval_ms)
    {
        return Ok(None);
    }
    Ok(Some(subset))
}

fn save_candle_snapshot(
    root: PathBuf,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    candles: Vec<Candle>,
) -> Result<(), String> {
    let candles = trim_cached_candles(normalize_candles(candles));
    if candles.is_empty() {
        return Ok(());
    }
    // Only claim completeness through the last candle when the snapshot is
    // actually contiguous; a gapped vec must not certify coverage across its
    // hole (see `candles_cover_range`).
    let contiguous = candle_interval_ms(interval)
        .map(|interval_ms| !candles_have_interior_gap(&candles, interval_ms))
        .unwrap_or(true);
    let complete_through_ms = if contiguous {
        candles.last().map(|candle| candle.open_time)
    } else {
        None
    };
    save_json(
        &root,
        "candles",
        &candle_key(source, symbol, interval),
        now_ms(),
        complete_through_ms,
        &candles,
    )
}

fn merge_candle_page_into_dir(
    root: &Path,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    candles: Vec<Candle>,
) -> Result<(), String> {
    let mut merged =
        match load_json::<Vec<Candle>>(root, "candles", &candle_key(source, symbol, interval)) {
            Ok(Some(cached)) => cached.payload,
            Ok(None) | Err(_) => Vec::new(),
        };
    merged.extend(candles);
    save_candle_snapshot(root.to_path_buf(), source, symbol, interval, merged)
}

fn load_fresh_watchlist_contexts_from_dir(
    root: &Path,
    symbols: &[String],
    now_ms: u64,
) -> Result<Option<HashMap<String, WatchlistContext>>, String> {
    let mut map = HashMap::new();
    let mut seen = HashSet::new();

    for symbol in symbols {
        if !seen.insert(symbol) {
            continue;
        }
        let Some(cached) = load_json::<WatchlistContext>(
            root,
            "watchlist_contexts",
            std::slice::from_ref(symbol),
        )?
        else {
            return Ok(None);
        };
        if now_ms.saturating_sub(cached.fetched_at_ms) > WATCHLIST_CONTEXT_FRESH_MS {
            return Ok(None);
        }
        map.insert(symbol.clone(), cached.payload);
    }

    Ok(Some(map))
}

fn candles_cover_range(
    candles: &[Candle],
    interval: &str,
    start_time: u64,
    end_time: u64,
    complete_through_ms: Option<u64>,
) -> bool {
    let Some(first) = candles.first() else {
        return false;
    };
    let Some(last) = candles.last() else {
        return false;
    };
    if first.open_time > start_time {
        return false;
    }

    let cached_tail = complete_through_ms
        .unwrap_or(last.open_time)
        .max(last.open_time);
    let required_tail = candle_interval_ms(interval)
        .map(|interval_ms| end_time.saturating_sub(interval_ms.saturating_mul(2)))
        .unwrap_or(end_time);
    cached_tail >= required_tail
}

fn trim_cached_candles(mut candles: Vec<Candle>) -> Vec<Candle> {
    if candles.len() > MAX_CACHED_CANDLES {
        let remove = candles.len() - MAX_CACHED_CANDLES;
        candles.drain(0..remove);
    }
    candles
}

fn candle_key(source: ChartBackfillSource, symbol: &str, interval: &str) -> Vec<String> {
    vec![
        source_key(source).to_string(),
        symbol.to_string(),
        interval.to_string(),
    ]
}

fn source_key(source: ChartBackfillSource) -> &'static str {
    match source {
        ChartBackfillSource::Hyperliquid => "hyperliquid",
        ChartBackfillSource::Hydromancer => "hydromancer",
        ChartBackfillSource::Schwab => "schwab",
    }
}

fn candle_interval_ms(interval: &str) -> Option<u64> {
    Some(match interval {
        "1s" => 1_000,
        "1m" => 60_000,
        "3m" => 3 * 60_000,
        "5m" => 5 * 60_000,
        "15m" => 15 * 60_000,
        "30m" => 30 * 60_000,
        "1h" => 60 * 60_000,
        "2h" => 2 * 60 * 60_000,
        "4h" => 4 * 60 * 60_000,
        "8h" => 8 * 60 * 60_000,
        "12h" => 12 * 60 * 60_000,
        "1d" => 24 * 60 * 60_000,
        "3d" => 3 * 24 * 60 * 60_000,
        "1w" => 7 * 24 * 60 * 60_000,
        "1M" => 31 * 24 * 60 * 60_000,
        _ => return None,
    })
}

fn cache_root() -> Result<PathBuf, String> {
    config::api_cache_dir().ok_or_else(|| "platform cache directory is unavailable".to_string())
}

// ---------------------------------------------------------------------------
// Background cache writer
//
// Cache writes serialize candle vectors and `fsync` files, and they fire from
// hot paths: candle snapshots persist on every websocket tick for charts with
// a secondary series, and watchlist contexts persist one file per symbol. None
// of that may block the iced update thread or a tokio worker, so every write is
// handed to a single dedicated thread. Routing merges through the same thread
// also serializes the read-modify-write so concurrent fetches of the same key
// can't clobber each other. Bursts are coalesced: a plain save made redundant
// by a later save/removal of the same file is dropped before touching disk.
// ---------------------------------------------------------------------------

enum CacheWrite {
    SaveCandles {
        root: PathBuf,
        source: ChartBackfillSource,
        symbol: String,
        interval: String,
        candles: Vec<Candle>,
    },
    MergeCandles {
        root: PathBuf,
        source: ChartBackfillSource,
        symbol: String,
        interval: String,
        candles: Vec<Candle>,
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
            } => json_path(root, "candles", &candle_key(*source, symbol, interval)),
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
            } => {
                let _ = save_candle_snapshot(root, source, &symbol, &interval, candles);
            }
            CacheWrite::MergeCandles {
                root,
                source,
                symbol,
                interval,
                candles,
            } => {
                let _ = merge_candle_page_into_dir(&root, source, &symbol, &interval, candles);
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
    let targets: Vec<PathBuf> = batch.iter().map(CacheWrite::target).collect();
    (0..batch.len())
        .map(|i| {
            if !batch[i].is_plain_save() {
                return true;
            }
            !((i + 1)..batch.len()).any(|j| targets[j] == targets[i] && batch[j].supersedes_save())
        })
        .collect()
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

fn enqueue(job: CacheWrite) {
    match cache_writer() {
        CacheWriter::Background(tx) => {
            let _ = tx.send(job);
        }
        CacheWriter::Inline => job.run(),
    }
}

fn load_json<T: DeserializeOwned>(
    root: &Path,
    namespace: &str,
    key: &[String],
) -> Result<Option<CachedPayload<T>>, String> {
    let path = json_path(root, namespace, key);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "read {} failed: {e}",
                config::user_config_path(&path)
            ));
        }
    };
    let envelope: CacheEnvelope<T> = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse {} failed: {e}", config::user_config_path(&path)))?;
    if envelope.schema_version != CACHE_SCHEMA_VERSION
        || envelope.namespace != namespace
        || envelope.key != key
    {
        return Ok(None);
    }
    Ok(Some(CachedPayload {
        fetched_at_ms: envelope.fetched_at_ms,
        complete_through_ms: envelope.complete_through_ms,
        payload: envelope.payload,
    }))
}

fn save_json<T: Serialize>(
    root: &Path,
    namespace: &str,
    key: &[String],
    fetched_at_ms: u64,
    complete_through_ms: Option<u64>,
    payload: &T,
) -> Result<(), String> {
    let (path, bytes) = envelope_bytes(
        root,
        namespace,
        key,
        fetched_at_ms,
        complete_through_ms,
        payload,
    )?;
    write_bytes_atomic(&path, &bytes)
}

/// Serialize a payload into its cache envelope, returning the destination path
/// and the encoded bytes. Cheap enough to run on the caller's thread; the
/// blocking write that follows is what gets handed to the cache writer.
fn envelope_bytes<T: Serialize>(
    root: &Path,
    namespace: &str,
    key: &[String],
    fetched_at_ms: u64,
    complete_through_ms: Option<u64>,
    payload: &T,
) -> Result<(PathBuf, Vec<u8>), String> {
    let path = json_path(root, namespace, key);
    let envelope = CacheEnvelope {
        schema_version: CACHE_SCHEMA_VERSION,
        namespace: namespace.to_string(),
        key: key.to_vec(),
        fetched_at_ms,
        complete_through_ms,
        payload,
    };
    let bytes = serde_json::to_vec(&envelope)
        .map_err(|e| format!("serialize {} failed: {e}", config::user_config_path(&path)))?;
    Ok((path, bytes))
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "cache path {} has no parent",
            config::user_config_path(path)
        ));
    };
    fs::create_dir_all(parent)
        .map_err(|e| format!("create {} failed: {e}", config::user_config_path(parent)))?;

    let temp_path = temp_json_path(path);
    let write_result = (|| -> Result<(), String> {
        let mut file = open_temp_file(&temp_path)?;
        file.write_all(bytes)
            .map_err(|e| format!("write {} failed: {e}", config::user_config_path(&temp_path)))?;
        file.sync_all()
            .map_err(|e| format!("sync {} failed: {e}", config::user_config_path(&temp_path)))?;
        drop(file);
        replace_with_temp(&temp_path, path)?;
        sync_parent_dir(path);
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn open_temp_file(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options
        .open(path)
        .map_err(|e| format!("create {} failed: {e}", config::user_config_path(path)))
}

fn remove_json_file(path: PathBuf) -> Result<(), String> {
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "remove {} failed: {e}",
            config::user_config_path(&path)
        )),
    }
}

fn replace_with_temp(temp_path: &Path, path: &Path) -> Result<(), String> {
    match fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(first_error) if cfg!(windows) && path.exists() => {
            fs::remove_file(path)
                .map_err(|e| format!("replace {} failed: {e}", config::user_config_path(path)))?;
            fs::rename(temp_path, path).map_err(|e| {
                format!(
                    "replace {} failed after removing old cache file: {e}; original error: {first_error}",
                    config::user_config_path(path)
                )
            })
        }
        Err(e) => Err(format!(
            "replace {} failed: {e}",
            config::user_config_path(path)
        )),
    }
}

fn sync_parent_dir(path: &Path) {
    if let Some(parent) = path.parent()
        && let Ok(dir) = File::open(parent)
    {
        let _ = dir.sync_all();
    }
}

fn json_path(root: &Path, namespace: &str, key: &[String]) -> PathBuf {
    let mut path = root.join(cache_component(namespace));
    for part in key {
        path.push(cache_component(part));
    }
    path.set_extension("json");
    path
}

fn temp_json_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "cache.json".into());
    file_name.push(format!(".tmp.{pid}.{nanos}.{counter}"));
    path.with_file_name(file_name)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn cache_component(input: &str) -> String {
    let mut component = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            component.push(byte as char);
        } else {
            component.push('_');
            component.push_str(&format!("{byte:02x}"));
        }
    }
    if component.is_empty() {
        "_".to_string()
    } else {
        component
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cache_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("kerosene-api-cache-{name}-{nanos}"))
    }

    #[test]
    fn coalesces_saves_superseded_by_a_later_save_to_the_same_path() {
        let path_a = PathBuf::from("/tmp/cache/a.json");
        let path_b = PathBuf::from("/tmp/cache/b.json");
        let batch = vec![
            CacheWrite::SaveBytes {
                path: path_a.clone(),
                bytes: vec![1],
            },
            CacheWrite::SaveBytes {
                path: path_b,
                bytes: vec![2],
            },
            CacheWrite::SaveBytes {
                path: path_a,
                bytes: vec![3],
            },
        ];
        // The first save to path_a is redundant; the later one wins.
        assert_eq!(writes_to_run(&batch), vec![false, true, true]);
    }

    #[test]
    fn remove_supersedes_earlier_save_but_merge_does_not() {
        let path = PathBuf::from("/tmp/cache/candles.json");
        let removal = vec![
            CacheWrite::SaveBytes {
                path: path.clone(),
                bytes: vec![1],
            },
            CacheWrite::Remove { path },
        ];
        assert_eq!(writes_to_run(&removal), vec![false, true]);

        // A merge reads the saved file, so an earlier save to the same key must
        // still run even though it shares the target path.
        let merge = vec![
            CacheWrite::SaveCandles {
                root: PathBuf::from("/tmp/cache"),
                source: ChartBackfillSource::Hyperliquid,
                symbol: "BTC".to_string(),
                interval: "1m".to_string(),
                candles: Vec::new(),
            },
            CacheWrite::MergeCandles {
                root: PathBuf::from("/tmp/cache"),
                source: ChartBackfillSource::Hyperliquid,
                symbol: "BTC".to_string(),
                interval: "1m".to_string(),
                candles: Vec::new(),
            },
        ];
        assert_eq!(writes_to_run(&merge), vec![true, true]);
    }

    #[test]
    fn cache_component_escapes_path_separators_and_market_prefixes() {
        assert_eq!(cache_component("BTC"), "BTC");
        assert_eq!(cache_component("xyz:NVDA"), "xyz_3aNVDA");
        assert_eq!(cache_component("@107"), "_40107");
        assert_eq!(cache_component("../secret"), "_2e_2e_2fsecret");
    }

    #[test]
    fn candle_range_load_requires_cached_coverage() {
        let root = test_cache_dir("candles");
        let candles = vec![
            Candle::test_flat(1_000, 100.0),
            Candle::test_flat(61_000, 101.0),
            Candle::test_flat(121_000, 102.0),
        ];
        save_candle_snapshot(
            root.clone(),
            ChartBackfillSource::Hyperliquid,
            "BTC",
            "1m",
            candles,
        )
        .expect("snapshot save succeeds");

        let cached = load_candles_for_range_from_dir(
            &root,
            ChartBackfillSource::Hyperliquid,
            "BTC",
            "1m",
            61_000,
            181_000,
        )
        .expect("range load succeeds")
        .expect("range should be covered");
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].open_time, 61_000);

        let missing = load_candles_for_range_from_dir(
            &root,
            ChartBackfillSource::Hyperliquid,
            "BTC",
            "1m",
            0,
            301_000,
        )
        .expect("range load succeeds");
        assert!(missing.is_none());

        let _ = fs::remove_dir_all(root);
    }

    /// A snapshot whose stale head is stitched to a fresh tail (the reported
    /// 1m HYPE "4d ago -> 3h ago" gap). The recent tail is within lookback, so
    /// the old gate would serve the whole gapped vec; the fix serves only the
    /// trailing contiguous run.
    fn gapped_hype_snapshot(now_ms: u64) -> Vec<Candle> {
        let four_days = 4 * 24 * 60 * 60 * 1_000;
        let old_start = now_ms - four_days;
        let recent_start = now_ms - 180_000;
        vec![
            Candle::test_flat(old_start, 60.0),
            Candle::test_flat(old_start + 60_000, 60.0),
            Candle::test_flat(old_start + 120_000, 60.0),
            Candle::test_flat(recent_start, 70.0),
            Candle::test_flat(recent_start + 60_000, 70.0),
            Candle::test_flat(recent_start + 120_000, 70.0),
        ]
    }

    #[test]
    fn load_fresh_candles_serves_only_trailing_run_across_interior_gap() {
        let root = test_cache_dir("fresh-gap");
        let now_ms = 10_000_000_000;
        save_candle_snapshot(
            root.clone(),
            ChartBackfillSource::Hyperliquid,
            "HYPE",
            "1m",
            gapped_hype_snapshot(now_ms),
        )
        .expect("snapshot save succeeds");

        let served = load_fresh_candles_from_dir(
            &root,
            ChartBackfillSource::Hyperliquid,
            "HYPE",
            Timeframe::M1,
            now_ms,
        )
        .expect("load succeeds")
        .expect("recent tail is fresh");

        // Only the recent contiguous block survives; the phantom $60 head is gone.
        assert_eq!(served.len(), 3);
        assert_eq!(served[0].open_time, now_ms - 180_000);
        assert!(served.iter().all(|candle| candle.close == 70.0));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_fresh_candles_rejects_tail_older_than_display_window() {
        let root = test_cache_dir("fresh-stale-tail");
        let timeframe = Timeframe::H1;
        let last_time = 1_000_000;
        let now_ms = last_time + timeframe.cache_display_max_age_ms() + 1;
        save_candle_snapshot(
            root.clone(),
            ChartBackfillSource::Hyperliquid,
            "BTC",
            timeframe.api_str(),
            vec![Candle::test_flat(last_time, 100.0)],
        )
        .expect("snapshot save succeeds");

        let served = load_fresh_candles_from_dir(
            &root,
            ChartBackfillSource::Hyperliquid,
            "BTC",
            timeframe,
            now_ms,
        )
        .expect("load succeeds");

        assert!(served.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn candle_range_load_misses_across_interior_gap() {
        let root = test_cache_dir("range-gap");
        let now_ms = 10_000_000_000;
        let candles = gapped_hype_snapshot(now_ms);
        let span_start = candles[0].open_time;
        save_candle_snapshot(
            root.clone(),
            ChartBackfillSource::Hyperliquid,
            "HYPE",
            "1m",
            candles,
        )
        .expect("snapshot save succeeds");

        // Endpoints are covered, but the range spans the hole — it must MISS so
        // the caller refetches instead of serving a gapped subset.
        let spanning = load_candles_for_range_from_dir(
            &root,
            ChartBackfillSource::Hyperliquid,
            "HYPE",
            "1m",
            span_start,
            now_ms,
        )
        .expect("range load succeeds");
        assert!(spanning.is_none());

        // A range wholly inside the recent block is still served.
        let recent_only = load_candles_for_range_from_dir(
            &root,
            ChartBackfillSource::Hyperliquid,
            "HYPE",
            "1m",
            now_ms - 180_000,
            now_ms,
        )
        .expect("range load succeeds")
        .expect("recent block is covered");
        assert_eq!(recent_only.len(), 3);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gapped_snapshot_is_not_certified_complete_through_last() {
        let root = test_cache_dir("complete-gap");
        let now_ms = 10_000_000_000;
        save_candle_snapshot(
            root.clone(),
            ChartBackfillSource::Hyperliquid,
            "HYPE",
            "1m",
            gapped_hype_snapshot(now_ms),
        )
        .expect("snapshot save succeeds");

        let cached = load_json::<Vec<Candle>>(
            &root,
            "candles",
            &candle_key(ChartBackfillSource::Hyperliquid, "HYPE", "1m"),
        )
        .expect("load succeeds")
        .expect("snapshot exists");
        assert_eq!(cached.complete_through_ms, None);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn watchlist_context_cache_requires_all_symbols_fresh() {
        let root = test_cache_dir("contexts");
        let now = 1_000_000;
        let btc = WatchlistContext {
            funding: Some(0.001),
            prev_day_px: Some(100.0),
            day_vlm: Some(1_000.0),
        };
        save_json(
            &root,
            "watchlist_contexts",
            &["BTC".to_string()],
            now,
            None,
            &btc,
        )
        .expect("context save succeeds");

        let symbols = vec!["BTC".to_string()];
        let fresh = load_fresh_watchlist_contexts_from_dir(&root, &symbols, now + 1_000)
            .expect("context load succeeds")
            .expect("context should be fresh");
        assert!(fresh.contains_key("BTC"));

        let stale = load_fresh_watchlist_contexts_from_dir(
            &root,
            &symbols,
            now + WATCHLIST_CONTEXT_FRESH_MS + 1,
        )
        .expect("context load succeeds");
        assert!(stale.is_none());

        let missing_symbols = vec!["BTC".to_string(), "ETH".to_string()];
        let missing = load_fresh_watchlist_contexts_from_dir(&root, &missing_symbols, now + 1_000)
            .expect("context load succeeds");
        assert!(missing.is_none());

        let _ = fs::remove_dir_all(root);
    }
}
