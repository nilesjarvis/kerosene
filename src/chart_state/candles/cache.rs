use crate::api::{self, Candle};
use crate::timeframe::Timeframe;
use std::collections::{HashMap, VecDeque};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Candle Cache
// ---------------------------------------------------------------------------

type CandleCacheKey = (String, Timeframe);

const CANDLE_CACHE_CAPACITY: usize = 100;

pub(super) fn store_normalized_candles(
    cache: &mut HashMap<CandleCacheKey, Vec<Candle>>,
    order: &mut VecDeque<CandleCacheKey>,
    symbol: &str,
    timeframe: Timeframe,
    candles: Vec<Candle>,
) {
    let candles = api::normalize_candles(candles);
    if candles.is_empty() {
        return;
    }

    let key = (symbol.to_string(), timeframe);
    order.retain(|existing| existing != &key);
    cache.insert(key.clone(), candles);
    order.push_back(key);

    if order.len() > CANDLE_CACHE_CAPACITY
        && let Some(oldest) = order.pop_front()
    {
        cache.remove(&oldest);
    }
}

pub(super) fn get_fresh_cached_candles(
    cache: &mut HashMap<CandleCacheKey, Vec<Candle>>,
    order: &mut VecDeque<CandleCacheKey>,
    symbol: &str,
    timeframe: Timeframe,
    now_ms: u64,
) -> Option<Vec<Candle>> {
    let key = (symbol.to_string(), timeframe);
    let candles = cache.get(&key)?;
    let last_time = candles.last().map(|candle| candle.open_time).unwrap_or(0);

    if now_ms.saturating_sub(last_time) > timeframe.lookback_ms() {
        cache.remove(&key);
        order.retain(|existing| existing != &key);
        return None;
    }

    order.retain(|existing| existing != &key);
    order.push_back(key);

    Some(candles.clone())
}
