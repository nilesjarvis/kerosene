use crate::chart::CandlestickChart;
use crate::config::ChartCrosshairStyle;

#[test]
fn hud_safety_timeout_waits_until_cursor_leaves_chart() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 1_000);

    assert!(chart.hud_armed());
    assert!(!chart.hud_safety_timeout_due(60_000));

    chart.record_hud_activity(2_000, false);

    assert!(!chart.hud_safety_timeout_due(16_999));
    assert!(chart.hud_safety_timeout_due(17_000));
}

#[test]
fn hud_safety_timeout_clears_when_chart_is_safe() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 1_000);
    chart.record_hud_activity(2_000, false);
    chart.set_hud_armed_at(false, 3_000);

    assert!(!chart.hud_armed());
    assert!(!chart.hud_safety_timeout_due(60_000));
}
