use super::model::Candle;

pub fn is_valid_candle(candle: &Candle) -> bool {
    candle.open_time > 0
        && candle.close_time >= candle.open_time
        && candle.open.is_finite()
        && candle.high.is_finite()
        && candle.low.is_finite()
        && candle.close.is_finite()
        && candle.volume.is_finite()
        && candle.volume >= 0.0
        && candle.low <= candle.high
        && candle.low <= candle.open
        && candle.low <= candle.close
        && candle.high >= candle.open
        && candle.high >= candle.close
}

pub fn normalize_candles(mut candles: Vec<Candle>) -> Vec<Candle> {
    candles.retain(is_valid_candle);
    candles.sort_by_key(|candle| candle.open_time);

    let mut normalized: Vec<Candle> = Vec::with_capacity(candles.len());
    for candle in candles {
        if let Some(last) = normalized.last_mut()
            && last.open_time == candle.open_time
        {
            *last = candle;
            continue;
        }
        normalized.push(candle);
    }
    normalized
}
