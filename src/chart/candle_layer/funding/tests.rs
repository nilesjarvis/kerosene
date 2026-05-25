use super::*;
use crate::api::Candle;
use crate::chart::model::{
    DEFAULT_FUNDING_PANEL_HEIGHT, FUNDING_MODE_BUTTON_HEIGHT, FUNDING_MODE_BUTTON_Y_OFFSET,
};
use crate::chart::state::ChartState;
use crate::hydromancer_api::FundingRatePoint;

fn candle(open_time: u64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    }
}

fn chart_with_funding() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle(1_000), candle(61_000), candle(121_000)]);
    chart.set_funding_history(vec![FundingRatePoint {
        time_ms: 61_000,
        rate: 0.01,
    }]);
    chart
}

#[test]
fn zoomed_funding_range_maps_values_beyond_plot_without_clamping() {
    let chart = chart_with_funding();
    let state = ChartState {
        funding_y_scale: 0.5,
        ..ChartState::default()
    };

    let range = chart
        .funding_display_range(&state, 400.0, 12.0)
        .expect("funding range");
    let y = range.rate_to_y(0.01, 24.0, 80.0);

    assert!(
        y < 24.0,
        "zoomed funding point should map above the plot, got {y}"
    );
}

#[test]
fn funding_range_uses_offset_as_visible_center() {
    let chart = chart_with_funding();
    let state = ChartState {
        funding_y_offset: 0.002,
        ..ChartState::default()
    };

    let range = chart
        .funding_display_range(&state, 400.0, 12.0)
        .expect("funding range");

    assert!(((range.hi + range.lo) * 0.5 - 0.002).abs() < 1e-12);
}

#[test]
fn oversized_funding_range_is_rejected() {
    let chart = chart_with_funding();
    let state = ChartState::default();

    let range = chart.funding_display_range_from_max_abs(1.7e308, &state);

    assert!(range.is_none());
}

#[test]
fn invalid_funding_range_falls_back_to_finite_coordinates() {
    let range = FundingDisplayRange {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    };

    assert_eq!(range.rate_to_y(0.0, 24.0, 80.0), 52.0);
    assert_eq!(range.y_to_rate(52.0, 24.0, 80.0), 0.0);
}

#[test]
fn default_funding_plot_uses_space_behind_mode_button() {
    let button_bottom = FUNDING_MODE_BUTTON_Y_OFFSET + FUNDING_MODE_BUTTON_HEIGHT;
    let plot_h =
        DEFAULT_FUNDING_PANEL_HEIGHT - FUNDING_PLOT_TOP_PADDING - FUNDING_PLOT_BOTTOM_PADDING;

    assert!(FUNDING_PLOT_TOP_PADDING < button_bottom);
    assert!(plot_h >= 40.0, "default funding plot height was {plot_h}");
}
