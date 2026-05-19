use super::{Candle, fetch_candles};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const OUTCOME_VOLUME_LOOKBACK_MS: u64 = 24 * 60 * 60 * 1000;
const MAX_OUTCOME_VOLUME_SYMBOLS: usize = 250;
const OUTCOME_VOLUME_FETCH_CONCURRENCY: usize = 8;

pub async fn fetch_outcome_volumes_24h(
    mut symbols: Vec<String>,
) -> Result<HashMap<String, f64>, String> {
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }

    if symbols.len() > MAX_OUTCOME_VOLUME_SYMBOLS {
        symbols.truncate(MAX_OUTCOME_VOLUME_SYMBOLS);
    }

    let end_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock before Unix epoch: {e}"))?
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let start_time = end_time.saturating_sub(OUTCOME_VOLUME_LOOKBACK_MS);

    let results = stream::iter(symbols.into_iter())
        .map(|symbol| fetch_outcome_symbol_volume(symbol, start_time, end_time))
        .buffer_unordered(OUTCOME_VOLUME_FETCH_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

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
        .map(|candle| candle.volume)
        .filter(|volume| volume.is_finite() && *volume > 0.0)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(volume: f64) -> Candle {
        Candle {
            open_time: 0,
            close_time: 0,
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume,
        }
    }

    #[test]
    fn outcome_volume_from_candles_sums_positive_finite_contract_volume() {
        let candles = vec![
            candle(10.0),
            candle(f64::NAN),
            candle(-4.0),
            candle(5.5),
            candle(f64::INFINITY),
        ];

        assert_eq!(outcome_volume_from_candles(&candles), 15.5);
    }
}
