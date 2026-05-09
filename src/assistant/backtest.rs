use crate::api::fetch_candles;
use chrono::Utc;

mod model;
mod render;
mod simulation;

use model::{DrawdownDcaResult, PriceLookupResult};
pub(super) use render::{
    render_drawdown_result, render_hourly_dca_result, render_price_lookup_result,
};
use simulation::{simulate_drawdown_dca, simulate_hourly_dca};

// ---------------------------------------------------------------------------
// Price Lookup and Backtests
// ---------------------------------------------------------------------------

pub(super) fn days_to_ms(days: u32) -> u64 {
    u64::from(days)
        .saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
        .saturating_mul(1000)
}

pub(super) async fn lookup_latest_price(
    symbol: String,
    interval: String,
) -> Result<PriceLookupResult, String> {
    let now_ms = Utc::now().timestamp_millis() as u64;
    let start_ms = now_ms.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let candles = fetch_candles(symbol.clone(), interval.clone(), start_ms, now_ms).await?;
    let Some(last) = candles.last() else {
        return Err(format!("No candle data available for {symbol}"));
    };
    Ok(PriceLookupResult {
        symbol,
        interval,
        price: last.close,
        candle_time: Some(last.open_time),
        source: "candle_snapshot".to_string(),
    })
}

pub(super) async fn run_drawdown_dca_backtest(
    symbol: String,
    interval: String,
    lookback_days: u32,
    tranche_usd: f64,
    drawdown_pct: f64,
) -> Result<DrawdownDcaResult, String> {
    let now_ms = Utc::now().timestamp_millis() as u64;
    let start_ms = now_ms.saturating_sub(days_to_ms(lookback_days));
    let candles = fetch_candles(symbol.clone(), interval.clone(), start_ms, now_ms).await?;
    simulate_drawdown_dca(
        candles,
        symbol,
        interval,
        lookback_days,
        tranche_usd,
        drawdown_pct,
    )
}

pub(super) async fn run_hourly_dca_backtest(
    symbol: String,
    lookback_days: u32,
    tranche_usd: f64,
) -> Result<DrawdownDcaResult, String> {
    let interval = "1h".to_string();
    let now_ms = Utc::now().timestamp_millis() as u64;
    let start_ms = now_ms.saturating_sub(days_to_ms(lookback_days));
    let candles = fetch_candles(symbol.clone(), interval.clone(), start_ms, now_ms).await?;
    simulate_hourly_dca(candles, symbol, interval, lookback_days, tranche_usd)
}
