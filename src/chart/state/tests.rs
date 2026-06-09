use super::ChartState;
use crate::api::Candle;
use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartViewport, PRICE_AXIS_WIDTH};

mod export;
mod reset;

fn test_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.candles = (0..8)
        .map(|idx| {
            let idx = idx as f64;
            Candle::test_ohlcv(
                idx as u64 * 60_000,
                idx as u64 * 60_000 + 59_999,
                [100.0 + idx, 110.0 + idx, 95.0 + idx, 104.0 + idx],
                1000.0 + idx,
            )
        })
        .collect();
    chart
}
