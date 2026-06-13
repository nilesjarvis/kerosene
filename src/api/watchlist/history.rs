use super::super::Candle;
use super::super::candles::fetch_candles;
use crate::app_time::now_ms;
use crate::helpers::finite_value;
use std::collections::HashMap;

pub async fn fetch_watchlist_history(
    symbols: Vec<String>,
) -> Result<HashMap<String, (f64, f64, f64)>, String> {
    fetch_symbol_history(
        symbols,
        history_baselines,
        Some("No watchlist history available"),
    )
    .await
}

pub async fn fetch_screener_history(
    symbols: Vec<String>,
) -> Result<HashMap<String, (f64, f64)>, String> {
    fetch_symbol_history(
        symbols,
        screener_history_baselines,
        Some("No screener history available"),
    )
    .await
}

async fn fetch_symbol_history<T>(
    symbols: Vec<String>,
    baselines: impl Fn(Vec<Candle>, u64) -> Option<T>,
    all_failed_error: Option<&str>,
) -> Result<HashMap<String, T>, String> {
    let now_ms = now_ms();
    // Fetch enough 1m candles to compute history baselines from timestamps
    // instead of assuming the API always returns perfect contiguous minutes.
    let start_ms = now_ms.saturating_sub(65 * 60 * 1000);
    let mut map = HashMap::new();
    let requested = symbols.len();
    let mut failed = 0;
    let mut last_error = None;

    for sym in symbols {
        let res = fetch_candles(sym.clone(), "1m".to_string(), start_ms, now_ms).await;
        match res {
            Ok(candles) => {
                if let Some(values) = baselines(candles, now_ms) {
                    map.insert(sym, values);
                }
            }
            Err(error) => {
                failed += 1;
                last_error = Some(error);
            }
        }
    }

    finish_symbol_history(map, requested, failed, last_error, all_failed_error)
}

fn finish_symbol_history<T>(
    map: HashMap<String, T>,
    requested: usize,
    failed: usize,
    last_error: Option<String>,
    all_failed_error: Option<&str>,
) -> Result<HashMap<String, T>, String> {
    if requested > 0
        && failed == requested
        && map.is_empty()
        && let Some(error) = all_failed_error
    {
        return Err(last_error.unwrap_or_else(|| error.to_string()));
    }

    Ok(map)
}

fn history_baselines(mut candles: Vec<Candle>, now_ms: u64) -> Option<(f64, f64, f64)> {
    sort_history_candles(&mut candles)?;
    Some((
        history_baseline(&candles, now_ms, 5)?,
        history_baseline(&candles, now_ms, 30)?,
        history_baseline(&candles, now_ms, 60)?,
    ))
}

fn screener_history_baselines(mut candles: Vec<Candle>, now_ms: u64) -> Option<(f64, f64)> {
    sort_history_candles(&mut candles)?;
    Some((
        history_baseline(&candles, now_ms, 15)?,
        history_baseline(&candles, now_ms, 60)?,
    ))
}

fn sort_history_candles(candles: &mut [Candle]) -> Option<()> {
    if candles.is_empty() {
        return None;
    }

    candles.sort_by_key(|c| c.open_time);
    Some(())
}

fn history_baseline(candles: &[Candle], now_ms: u64, minutes_ago: u64) -> Option<f64> {
    let target = now_ms.saturating_sub(minutes_ago * 60 * 1000);
    candles
        .iter()
        .rev()
        .find(|c| c.open_time <= target)
        .or_else(|| candles.first())
        .map(|c| c.open)
        .and_then(finite_value)
}

#[cfg(test)]
mod tests;
