use super::ChartState;
use crate::api::Candle;
use crate::chart::{CANDLE_GAP_RATIO, CandlestickChart, ChartViewport, PRICE_AXIS_WIDTH};

mod export;
mod reset;

fn test_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.candles = (0..8)
        .map(|idx| Candle {
            open_time: idx * 60_000,
            close_time: idx * 60_000 + 59_999,
            open: 100.0 + idx as f64,
            high: 110.0 + idx as f64,
            low: 95.0 + idx as f64,
            close: 104.0 + idx as f64,
            volume: 1000.0 + idx as f64,
        })
        .collect();
    chart
}
