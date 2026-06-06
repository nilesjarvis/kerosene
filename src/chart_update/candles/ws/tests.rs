use super::*;
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

fn candle(open_time: u64, close: f64) -> Candle {
    Candle::test_ohlcv(
        open_time,
        open_time + 60_000,
        [close, close, close, close],
        1.0,
    )
}

fn last_close(terminal: &TradingTerminal, id: ChartId) -> Option<f64> {
    terminal
        .charts
        .get(&id)
        .expect("chart")
        .chart
        .candles
        .last()
        .map(|candle| candle.close)
}

#[test]
fn ws_candle_update_fans_out_to_matching_chart_instances() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();

    let mut first = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    first.chart.status = ChartStatus::Loaded;
    first.chart.set_candles(vec![candle(1_000, 100.0)]);

    let mut second = ChartInstance::new(2, "BTC".to_string(), Timeframe::H1);
    second.chart.status = ChartStatus::Loaded;
    second.chart.set_candles(vec![candle(1_000, 100.0)]);

    let mut different_timeframe = ChartInstance::new(3, "BTC".to_string(), Timeframe::M5);
    different_timeframe.chart.status = ChartStatus::Loaded;
    different_timeframe
        .chart
        .set_candles(vec![candle(1_000, 100.0)]);

    terminal.charts.insert(1, first);
    terminal.charts.insert(2, second);
    terminal.charts.insert(3, different_timeframe);

    let _task = terminal.apply_chart_ws_candle_update(
        1,
        "BTC".to_string(),
        "1h".to_string(),
        candle(2_000, 101.0),
    );

    assert_eq!(last_close(&terminal, 1), Some(101.0));
    assert_eq!(last_close(&terminal, 2), Some(101.0));
    assert_eq!(last_close(&terminal, 3), Some(100.0));
}
