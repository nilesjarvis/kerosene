use super::{Candle, fetch_candles};
use crate::helpers::positive_finite_value;

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const OUTCOME_VOLUME_LOOKBACK_MS: u64 = 24 * 60 * 60 * 1000;

pub async fn fetch_outcome_volumes_24h(
    symbols: Vec<String>,
) -> Result<HashMap<String, f64>, String> {
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }

    let end_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock before Unix epoch: {e}"))?
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let start_time = end_time.saturating_sub(OUTCOME_VOLUME_LOOKBACK_MS);

    let fetches = symbols
        .into_iter()
        .map(|symbol| fetch_outcome_symbol_volume(symbol, start_time, end_time));
    let results = futures::future::join_all(fetches).await;

    let mut volumes = HashMap::new();
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok((symbol, volume)) => {
                volumes.insert(symbol, volume);
            }
            Err(error) => errors.push(error),
        }
    }

    if volumes.is_empty()
        && let Some(error) = errors.into_iter().next()
    {
        return Err(error);
    }

    Ok(volumes)
}

async fn fetch_outcome_symbol_volume(
    symbol: String,
    start_time: u64,
    end_time: u64,
) -> Result<(String, f64), String> {
    let candles = fetch_candles(symbol.clone(), "1h".to_string(), start_time, end_time).await?;
    let volume = outcome_volume_from_candles(&candles);
    Ok((symbol, volume))
}

fn outcome_volume_from_candles(candles: &[Candle]) -> f64 {
    candles
        .iter()
        .filter_map(|candle| positive_finite_value(candle.volume))
        .sum()
}

#[cfg(test)]
mod tests;
