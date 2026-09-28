use crate::api::{
    Candle, CandleFetchPolicy, ChartCandleFetchRequest, fetch_chart_backfill_candles,
};
use crate::config::{ChartBackfillSource, LiveWatchlistEmaConfig};
use crate::helpers::positive_percent_change;

/// A candle-close EMA snapshot. While its last candle is forming, replace that
/// candle's close with the current mid without accumulating repeated mid ticks.
#[derive(Debug, Clone)]
pub(crate) struct WatchlistEmaSample {
    pub(crate) fetched_at_ms: u64,
    ema: f64,
    last_close: f64,
    close_time: u64,
    live_weight: f64,
}

impl WatchlistEmaSample {
    pub(crate) fn distance(&self, mid: Option<f64>, now_ms: u64) -> Option<f64> {
        if self.candle_has_closed(now_ms) || now_ms.saturating_sub(self.fetched_at_ms) > 120_000 {
            return None;
        }
        let mid = mid?;
        let ema = self.ema + (mid - self.last_close) * self.live_weight;
        positive_percent_change(Some(mid), Some(ema))
    }

    pub(crate) fn candle_has_closed(&self, now_ms: u64) -> bool {
        now_ms > self.close_time
    }
}

pub(crate) async fn fetch_watchlist_ema(
    symbol: String,
    settings: LiveWatchlistEmaConfig,
) -> Result<WatchlistEmaSample, String> {
    let now_ms = crate::app_time::now_ms();
    // Warm up the SMA-seeded EMA with up to five periods of candle history.
    let count = settings.period.saturating_mul(5).clamp(5, 5_000) as u64;
    let lookback = settings.interval().duration_ms().saturating_mul(count);
    let candles = fetch_chart_backfill_candles(ChartCandleFetchRequest {
        source: ChartBackfillSource::Hyperliquid,
        hydromancer_api_key: Default::default(),
        coin: symbol,
        interval: settings.timeframe.clone(),
        start_time: now_ms.saturating_sub(lookback),
        end_time: now_ms,
        policy: CandleFetchPolicy::NetworkOnly,
    })
    .await?;
    WatchlistEmaSample::from_candles(candles, &settings, now_ms)
}

impl WatchlistEmaSample {
    pub(crate) fn from_candles(
        candles: Vec<Candle>,
        settings: &LiveWatchlistEmaConfig,
        now_ms: u64,
    ) -> Result<WatchlistEmaSample, String> {
        let mut candles = crate::api::normalize_candles(candles);
        candles.retain(|candle| candle.open_time <= now_ms);
        if settings.period == 0 || candles.len() < settings.period {
            return Err("Not enough candle history for this EMA".to_string());
        }
        if candles.iter().any(|candle| candle.close <= 0.0) {
            return Err("Invalid candle prices for this EMA".to_string());
        }
        let last = candles.last().ok_or("No EMA candle history")?;
        if last.close_time < now_ms.saturating_sub(60_000) {
            return Err("EMA candle history is stale".to_string());
        }
        let ema = crate::chart::calculate_ema(&candles, settings.period)
            .last()
            .map(|(_, ema)| *ema)
            .filter(|ema| ema.is_finite() && *ema > 0.0)
            .ok_or("EMA unavailable")?;
        Ok(WatchlistEmaSample {
            fetched_at_ms: now_ms,
            ema,
            last_close: last.close,
            close_time: last.close_time,
            live_weight: if candles.len() == settings.period {
                1.0 / settings.period as f64
            } else {
                2.0 / (settings.period as f64 + 1.0)
            },
        })
    }
}

#[cfg(test)]
mod tests;
