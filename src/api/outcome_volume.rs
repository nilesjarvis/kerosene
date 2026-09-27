use super::{Candle, fetch_candles};
use crate::helpers::positive_finite_value;
use futures::{StreamExt, stream};

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const OUTCOME_VOLUME_LOOKBACK_MS: u64 = 24 * 60 * 60 * 1000;
// Leave capacity in the shared read queue for charts and other market widgets.
const MAX_CONCURRENT_OUTCOME_VOLUME_FETCHES: usize = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct OutcomeVolume24h {
    pub(crate) contract: f64,
    pub(crate) notional: f64,
}

pub async fn fetch_outcome_volumes_24h(
    symbols: Vec<String>,
) -> Result<HashMap<String, OutcomeVolume24h>, String> {
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }

    let end_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock before Unix epoch: {e}"))?
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let start_time = end_time.saturating_sub(OUTCOME_VOLUME_LOOKBACK_MS);

    fetch_outcome_volumes_with(symbols, |symbol| {
        fetch_outcome_symbol_volume(symbol, start_time, end_time)
    })
    .await
}

async fn fetch_outcome_volumes_with<F, Fut>(
    symbols: Vec<String>,
    fetch: F,
) -> Result<HashMap<String, OutcomeVolume24h>, String>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<(String, OutcomeVolume24h), String>>,
{
    // Keep futures in this task so aborting a hidden widget drops queued and
    // in-flight reads, including waits for the shared API budget.
    let mut results = stream::iter(symbols)
        .map(fetch)
        .buffer_unordered(MAX_CONCURRENT_OUTCOME_VOLUME_FETCHES);

    let mut volumes = HashMap::new();
    let mut first_error = None;
    while let Some(result) = results.next().await {
        match result {
            Ok((symbol, volume)) => {
                volumes.insert(symbol, volume);
            }
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }

    if volumes.is_empty()
        && let Some(error) = first_error
    {
        return Err(error);
    }

    Ok(volumes)
}

async fn fetch_outcome_symbol_volume(
    symbol: String,
    start_time: u64,
    end_time: u64,
) -> Result<(String, OutcomeVolume24h), String> {
    let candles = fetch_candles(symbol.clone(), "1h".to_string(), start_time, end_time).await?;
    let volume = outcome_volume_from_candles(&candles);
    Ok((symbol, volume))
}

fn outcome_volume_from_candles(candles: &[Candle]) -> OutcomeVolume24h {
    let mut volume = OutcomeVolume24h::default();
    for candle in candles {
        let Some(contract) = positive_finite_value(candle.volume) else {
            continue;
        };
        volume.contract += contract;
        if let Some(price) = positive_finite_value(candle.close) {
            volume.notional += contract * price;
        }
    }
    volume
}

#[cfg(test)]
mod tests;
