use super::super::Candle;
use super::super::candles::fetch_candles;
use crate::helpers::finite_value;
use std::collections::HashMap;

pub async fn fetch_watchlist_history(
    symbols: Vec<String>,
) -> Result<HashMap<String, (f64, f64, f64)>, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Fetch enough 1m candles to compute 5m/30m/1h baselines from timestamps
    // instead of assuming the API always returns perfect contiguous minutes.
    let start_ms = now_ms.saturating_sub(65 * 60 * 1000);

    let mut map = HashMap::new();

    for sym in symbols {
        let res = fetch_candles(sym.clone(), "1m".to_string(), start_ms, now_ms).await;
        if let Ok(candles) = res {
            if candles.is_empty() {
                continue;
            }

            let Some((px_5m, px_30m, px_1h)) = history_baselines(candles, now_ms) else {
                continue;
            };

            map.insert(sym, (px_5m, px_30m, px_1h));
        }
    }

    Ok(map)
}

fn history_baselines(mut candles: Vec<Candle>, now_ms: u64) -> Option<(f64, f64, f64)> {
    if candles.is_empty() {
        return None;
    }

    candles.sort_by_key(|c| c.open_time);
    let baseline = |minutes_ago: u64| {
        let target = now_ms.saturating_sub(minutes_ago * 60 * 1000);
        candles
            .iter()
            .rev()
            .find(|c| c.open_time <= target)
            .or_else(|| candles.first())
            .map(|c| c.open)
            .and_then(finite_value)
    };

    Some((baseline(5)?, baseline(30)?, baseline(60)?))
}

#[cfg(test)]
mod tests;
