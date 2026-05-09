use super::model::{DrawdownDcaResult, PriceLookupResult};

pub(in crate::assistant) fn render_drawdown_result(result: DrawdownDcaResult) -> String {
    format!(
        "Backtest: drawdown DCA\nSymbol: {}\nInterval: {}\nLookback: {} days\nTrigger: {:.2}%\nTranche: ${:.2}\nEntries: {}\nInvested: ${:.2}\nUnits: {:.8}\nEnd price: ${:.6}\nEnding value: ${:.2}\nPnL: ${:.2}\nROI: {:.2}%",
        result.symbol,
        result.interval,
        result.lookback_days,
        result.drawdown_pct,
        result.tranche_usd,
        result.entries,
        result.invested_usd,
        result.units,
        result.end_price,
        result.ending_value_usd,
        result.pnl_usd,
        result.roi_pct,
    )
}

pub(in crate::assistant) fn render_hourly_dca_result(result: DrawdownDcaResult) -> String {
    format!(
        "Backtest: hourly DCA\nSymbol: {}\nInterval: {}\nLookback: {} days\nTranche: ${:.2}\nEntries: {}\nInvested: ${:.2}\nUnits: {:.8}\nEnd price: ${:.6}\nEnding value: ${:.2}\nPnL: ${:.2}\nROI: {:.2}%",
        result.symbol,
        result.interval,
        result.lookback_days,
        result.tranche_usd,
        result.entries,
        result.invested_usd,
        result.units,
        result.end_price,
        result.ending_value_usd,
        result.pnl_usd,
        result.roi_pct,
    )
}

pub(in crate::assistant) fn render_price_lookup_result(result: &PriceLookupResult) -> String {
    let ts = result
        .candle_time
        .and_then(|t| i64::try_from(t).ok())
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "live".to_string());
    format!(
        "{} price: ${:.6}\nSource: {}\nInterval: {}\nAs of: {}",
        result.symbol, result.price, result.source, result.interval, ts
    )
}
