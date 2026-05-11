use crate::api::Candle;
use crate::chart::model::CandlestickChart;
use crate::chart::state::ChartState;

fn candle_at(open_time: u64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: 10.0,
        high: 11.0,
        low: 9.0,
        close: 10.0,
        volume: 1.0,
    }
}

fn chart_with(candles: usize) -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    let mut data = Vec::with_capacity(candles);
    for i in 0..candles {
        data.push(candle_at(1_000 + (i as u64) * 60_000));
    }
    chart.set_candles(data);
    chart
}

#[test]
fn x_to_candle_index_returns_none_when_chart_is_empty() {
    let chart = CandlestickChart::new(1);
    let state = ChartState::default();
    assert_eq!(chart.x_to_candle_index(100.0, &state, 400.0), None);
}

#[test]
fn x_to_candle_index_returns_none_for_degenerate_bounds() {
    let chart = chart_with(3);
    let state = ChartState::default();
    assert_eq!(chart.x_to_candle_index(50.0, &state, 0.0), None);

    let zero_width_state = ChartState {
        candle_width: 0.0,
        ..ChartState::default()
    };
    assert_eq!(
        chart.x_to_candle_index(50.0, &zero_width_state, 400.0),
        None
    );
}

#[test]
fn x_to_candle_index_locates_the_rightmost_candle_at_the_chart_edge() {
    let chart = chart_with(3);
    let state = ChartState::default();
    let chart_w = 400.0;
    // The last candle sits at chart_w - step/2; sampling its center returns
    // the last index.
    let step = state.candle_width * (1.0 + crate::chart::CANDLE_GAP_RATIO);
    let last_center = chart_w - step * 0.5;
    assert_eq!(
        chart.x_to_candle_index(last_center, &state, chart_w),
        Some(2)
    );
}

#[test]
fn x_to_candle_index_returns_none_past_the_right_edge() {
    let chart = chart_with(3);
    let state = ChartState::default();
    assert_eq!(chart.x_to_candle_index(400.0, &state, 400.0), None);
}
