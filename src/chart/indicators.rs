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

// ---------------------------------------------------------------------------
// Leledc Exhaustion Levels
// ---------------------------------------------------------------------------
// Port of the open-source TradingView study "Leledc levels (IS)" by InSilico
// (https://www.tradingview.com/script/jB2a9GAV-Leledc-levels-IS/).

/// Swing window a bar must top/bottom to count as exhaustion ("Exhaustions swing length").
pub const LELEDC_SWING_LENGTH: usize = 40;
/// Directional closes that must accumulate before exhaustion can fire ("Exhaustion bar count").
pub const LELEDC_BAR_COUNT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeledcSignal {
    /// Buyer exhaustion at a swing high; the bar's high becomes resistance.
    Bearish,
    /// Seller exhaustion at a swing low; the bar's low becomes support.
    Bullish,
}

/// Horizontal level set by an exhaustion bar, active over a candle index range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeledcLevel {
    /// Candle index of the exhaustion bar that set the level.
    pub start: usize,
    /// Last candle index (inclusive) before the level is replaced.
    pub end: usize,
    /// Level price: exhaustion bar high for resistance, low for support.
    pub price: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LeledcLevels {
    /// Exhaustion bars as (candle index, direction), in chronological order.
    pub signals: Vec<(usize, LeledcSignal)>,
    pub resistance: Vec<LeledcLevel>,
    pub support: Vec<LeledcLevel>,
}

pub fn calculate_leledc(candles: &[Candle], swing_length: usize, bar_count: usize) -> LeledcLevels {
    let mut levels = LeledcLevels::default();
    if candles.is_empty() || swing_length == 0 {
        return levels;
    }
    let last_index = candles.len() - 1;
    // Running counts of bars closing above/below the close four bars back.
    let mut bull_closes: usize = 0;
    let mut bear_closes: usize = 0;

    for (i, candle) in candles.iter().enumerate() {
        if let Some(prior) = i.checked_sub(4).map(|j| &candles[j]) {
            if candle.close > prior.close {
                bull_closes += 1;
            }
            if candle.close < prior.close {
                bear_closes += 1;
            }
        }

        let window = &candles[(i + 1).saturating_sub(swing_length)..=i];
        if bull_closes > bar_count
            && candle.close < candle.open
            && window.iter().all(|c| c.high <= candle.high)
        {
            bull_closes = 0;
            levels.signals.push((i, LeledcSignal::Bearish));
            if let Some(previous) = levels.resistance.last_mut() {
                previous.end = i - 1;
            }
            levels.resistance.push(LeledcLevel {
                start: i,
                end: last_index,
                price: candle.high,
            });
        } else if bear_closes > bar_count
            && candle.close > candle.open
            && window.iter().all(|c| c.low >= candle.low)
        {
            bear_closes = 0;
            levels.signals.push((i, LeledcSignal::Bullish));
            if let Some(previous) = levels.support.last_mut() {
                previous.end = i - 1;
            }
            levels.support.push(LeledcLevel {
                start: i,
                end: last_index,
                price: candle.low,
            });
        }
    }
    levels
}
