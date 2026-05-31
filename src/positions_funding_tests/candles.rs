use crate::app_state::TradingTerminal;
use crate::timeframe::Timeframe;

#[test]
fn candle_fetch_request_overlaps_cached_candle_refresh() {
    let last_candle_ms = 10 * 60 * 60 * 1000;
    let request = TradingTerminal::build_candle_fetch_request(
        7,
        "BTC",
        Timeframe::H1,
        crate::config::ChartBackfillSource::Hyperliquid,
        Some(last_candle_ms),
        0,
    );

    assert_eq!(request.chart_id, 7);
    assert_eq!(request.symbol, "BTC");
    assert_eq!(request.timeframe, Timeframe::H1);
    assert_eq!(
        request.source,
        crate::config::ChartBackfillSource::Hyperliquid
    );
    assert_eq!(
        request.start_ms,
        last_candle_ms - Timeframe::H1.duration_ms() * 2
    );
    assert_eq!(request.attempt, 0);
}

#[test]
fn candle_fetch_retry_delay_backoff_is_bounded() {
    assert_eq!(TradingTerminal::candle_fetch_retry_delay_ms(0), 0);
    assert_eq!(TradingTerminal::candle_fetch_retry_delay_ms(1), 1_000);
    assert_eq!(TradingTerminal::candle_fetch_retry_delay_ms(2), 3_000);
    assert_eq!(TradingTerminal::candle_fetch_retry_delay_ms(3), 8_000);
    assert_eq!(TradingTerminal::candle_fetch_retry_delay_ms(10), 8_000);
}
