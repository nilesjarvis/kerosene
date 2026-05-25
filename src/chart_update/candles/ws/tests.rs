use super::*;
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

fn candle(open_time: u64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 60_000,
        open: close,
        high: close,
        low: close,
        close,
        volume: 1.0,
    }
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

    assert_eq!(
        terminal
            .charts
            .get(&1)
            .expect("first chart")
            .chart
            .candles
            .last()
            .map(|candle| candle.close),
        Some(101.0)
    );
    assert_eq!(
        terminal
            .charts
            .get(&2)
            .expect("second chart")
            .chart
            .candles
            .last()
            .map(|candle| candle.close),
        Some(101.0)
    );
    assert_eq!(
        terminal
            .charts
            .get(&3)
            .expect("different timeframe")
            .chart
            .candles
            .last()
            .map(|candle| candle.close),
        Some(100.0)
    );
}
