use crate::api::{ExchangeSymbolsPayload, WatchlistContext};
use crate::config;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod candles;
mod storage;
mod writer;

pub(crate) use candles::{
    cache_eligible, cache_requires_exact_intervals, load_candles_for_range, load_fresh_candles,
    merge_candle_page, remove_candles, save_candles_snapshot,
};
pub(crate) use storage::write_bytes_atomic;

use storage::{envelope_bytes, load_json};
use writer::{CacheWrite, enqueue};

const EXCHANGE_SYMBOLS_FRESH_MS: u64 = 6 * 60 * 60 * 1000;
const WATCHLIST_CONTEXT_FRESH_MS: u64 = 15_000;

#[derive(Debug, Clone)]
pub(crate) struct CachedPayload<T> {
    pub(crate) fetched_at_ms: u64,
    pub(crate) complete_through_ms: Option<u64>,
    pub(crate) payload: T,
}

pub(crate) fn load_fresh_exchange_symbols(
    now_ms: u64,
) -> Result<Option<ExchangeSymbolsPayload>, String> {
    let Some(cached) = load_json::<ExchangeSymbolsPayload>(
        &cache_root()?,
        "market_metadata",
        &["exchange_symbols_v2".to_string()],
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
        &["exchange_symbols_v2".to_string()],
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

fn cache_root() -> Result<PathBuf, String> {
    config::api_cache_dir().ok_or_else(|| "platform cache directory is unavailable".to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
