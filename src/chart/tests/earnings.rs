use super::candle_at;
use crate::chart::{CandlestickChart, EarningsMarker};

fn chart_with_earnings_marker() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 110.0)]);
    chart.set_earnings_markers(vec![EarningsMarker {
        time_ms: 2_000,
        cik: 1_652_044,
        form: "8-K".to_string(),
        filing_date: "2026-04-29".to_string(),
        accession_number: "0001652044-26-000043".to_string(),
        primary_document: "goog-20260429.htm".to_string(),
        quarter_label: Some("Q1 2026".to_string()),
    }]);
    chart
}

#[test]
fn earnings_marker_hover_animation_eases_toward_target() {
    let mut chart = chart_with_earnings_marker();

    chart.set_earnings_marker_hover(Some(2_000));
    chart.advance_earnings_marker_hover_animation();

    assert!(chart.earnings_marker_hover_animation_active());
    assert!(chart.earnings_marker_hover_progress_for(2_000) > 0.0);

    chart.set_earnings_marker_hover(None);
    for _ in 0..20 {
        chart.advance_earnings_marker_hover_animation();
    }

    assert!(!chart.earnings_marker_hover_animation_active());
}
