use crate::api::Candle;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Moving Averages
// ---------------------------------------------------------------------------

pub fn calculate_sma(candles: &[Candle], period: usize) -> Vec<(u64, f64)> {
    if candles.len() < period || period == 0 {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(candles.len() - period + 1);
    let mut sum: f64 = candles.iter().take(period).map(|c| c.close).sum();
    result.push((candles[period - 1].open_time, sum / period as f64));

    for (i_idx, candle) in (period..).zip(candles.iter().skip(period)) {
        sum += candle.close - candles[i_idx - period].close;
        result.push((candle.open_time, sum / period as f64));
    }
    result
}

pub fn calculate_ema(candles: &[Candle], period: usize) -> Vec<(u64, f64)> {
    if candles.len() < period || period == 0 {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(candles.len() - period + 1);
    let k = 2.0 / (period as f64 + 1.0);

    let mut ema: f64 = candles.iter().take(period).map(|c| c.close).sum::<f64>() / period as f64;
    result.push((candles[period - 1].open_time, ema));

    for candle in candles.iter().skip(period) {
        ema = (candle.close - ema) * k + ema;
        result.push((candle.open_time, ema));
    }
    result
}
