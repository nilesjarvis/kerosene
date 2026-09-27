use crate::api::{
    Candle, candles_have_interior_gap, candles_have_interval_discontinuity, normalize_candles,
    trailing_contiguous_run_start, trailing_exact_run_start,
};
use crate::config::ChartBackfillSource;
use crate::timeframe::Timeframe;
use std::path::{Path, PathBuf};

use super::storage::{json_path, load_json, save_json};
use super::writer::{CacheWrite, enqueue};
use super::{cache_root, now_ms};

// The namespace change invalidates only legacy candle snapshots whose mutable
// tail may have been frozen as complete. Other API caches retain compatibility.
pub(super) const CANDLE_CACHE_NAMESPACE: &str = "candles_v2";
const MAX_CACHED_CANDLES: usize = 12_000;

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
    let Some(last_time) = candles.last().map(|candle| candle.close_time) else {
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
    let start = if cache_requires_exact_intervals(symbol, timeframe.api_str()) {
        trailing_exact_run_start(&candles, timeframe.duration_ms())
    } else {
        trailing_contiguous_run_start(&candles, timeframe.duration_ms())
    };
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
        observed_at_ms: now_ms(),
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
        observed_at_ms: now_ms(),
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
        CANDLE_CACHE_NAMESPACE,
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

fn load_candle_snapshot_from_dir(
    root: &Path,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
) -> Result<Option<Vec<Candle>>, String> {
    let Some(cached) = load_json::<Vec<Candle>>(
        root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(source, symbol, interval),
    )?
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
    let Some(cached) = load_json::<Vec<Candle>>(
        root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(source, symbol, interval),
    )?
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
    if let Some(interval_ms) = candle_interval_ms(interval) {
        let has_gap = if cache_requires_exact_intervals(symbol, interval) {
            candles_have_interval_discontinuity(&subset, interval_ms)
        } else {
            candles_have_interior_gap(&subset, interval_ms)
        };
        if has_gap {
            return Ok(None);
        }
    }
    Ok(Some(subset))
}

#[cfg(test)]
fn save_candle_snapshot(
    root: PathBuf,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    candles: Vec<Candle>,
) -> Result<(), String> {
    save_candle_snapshot_at(root, source, symbol, interval, candles, now_ms())
}

pub(super) fn save_candle_snapshot_at(
    root: PathBuf,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    candles: Vec<Candle>,
    observed_at_ms: u64,
) -> Result<(), String> {
    // A provider snapshot and every live chart vector can contain the active
    // bucket. Its close is mutable, so capturing it would freeze a partial
    // value and manufacture a discontinuity after restart.
    let closed = candles
        .into_iter()
        .filter(|candle| candle.close_time <= observed_at_ms)
        .collect();
    let candles = trim_cached_candles(normalize_candles(closed));
    if candles.is_empty() {
        return Ok(());
    }
    // Only claim completeness through the last candle when the snapshot is
    // actually contiguous; a gapped vec must not certify coverage across its
    // hole (see `candles_cover_range`).
    let contiguous = candle_interval_ms(interval)
        .map(|interval_ms| !candles_have_interval_discontinuity(&candles, interval_ms))
        .unwrap_or(true);
    let complete_through_ms = if contiguous {
        candles.last().map(|candle| candle.close_time)
    } else {
        None
    };
    save_json(
        &root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(source, symbol, interval),
        observed_at_ms,
        complete_through_ms,
        &candles,
    )
}

pub(super) fn merge_candle_page_into_dir(
    root: &Path,
    source: ChartBackfillSource,
    symbol: &str,
    interval: &str,
    candles: Vec<Candle>,
    observed_at_ms: u64,
) -> Result<(), String> {
    let mut merged = match load_json::<Vec<Candle>>(
        root,
        CANDLE_CACHE_NAMESPACE,
        &candle_key(source, symbol, interval),
    ) {
        Ok(Some(cached)) => cached.payload,
        Ok(None) | Err(_) => Vec::new(),
    };
    merged.extend(candles);
    save_candle_snapshot_at(
        root.to_path_buf(),
        source,
        symbol,
        interval,
        merged,
        observed_at_ms,
    )
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
    if first.open_time > start_time {
        return false;
    }

    let Some(cached_tail) = complete_through_ms else {
        return false;
    };
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

pub(super) fn candle_key(source: ChartBackfillSource, symbol: &str, interval: &str) -> Vec<String> {
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
    }
}

fn candle_interval_ms(interval: &str) -> Option<u64> {
    Timeframe::from_api_str_opt(interval)
        .filter(|timeframe| timeframe.uses_candle_backfill())
        .map(Timeframe::duration_ms)
}

pub(crate) fn cache_requires_exact_intervals(symbol: &str, interval: &str) -> bool {
    !symbol.starts_with('@')
        && !symbol.starts_with('#')
        && !symbol.contains('/')
        && interval != Timeframe::Mo1.api_str()
}

#[cfg(test)]
mod tests;
