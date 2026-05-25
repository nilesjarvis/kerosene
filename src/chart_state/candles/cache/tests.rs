use crate::api::Candle;
use crate::timeframe::Timeframe;

use super::*;
use std::collections::{HashMap, VecDeque};

mod fresh;
mod store;

fn candle(open_time: u64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 10.0,
    }
}

fn cache_key(symbol: &str, timeframe: Timeframe) -> CandleCacheKey {
    (symbol.to_string(), timeframe)
}

fn cached_candles_or_panic<'a>(
    cache: &'a HashMap<CandleCacheKey, Vec<Candle>>,
    symbol: &str,
    timeframe: Timeframe,
) -> &'a [Candle] {
    match cache.get(&cache_key(symbol, timeframe)) {
        Some(candles) => candles,
        None => panic!("missing cached candles for {symbol}"),
    }
}

fn fresh_candles_or_panic(
    cache: &mut HashMap<CandleCacheKey, Vec<Candle>>,
    order: &mut VecDeque<CandleCacheKey>,
    symbol: &str,
    timeframe: Timeframe,
    now_ms: u64,
) -> Vec<Candle> {
    match get_fresh_cached_candles(cache, order, symbol, timeframe, now_ms) {
        Some(candles) => candles,
        None => panic!("missing fresh cached candles for {symbol}"),
    }
}
