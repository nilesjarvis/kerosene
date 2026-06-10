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

#[test]
fn racing_hud_uses_hud_safety_timeout() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::RacingHud);
    chart.set_hud_armed_at(true, 1_000);
    chart.record_hud_activity(2_000, false);

    assert!(chart.hud_order_submission_enabled());
    assert!(chart.hud_safety_timeout_due(17_000));
}

#[test]
fn hud_idle_warning_fires_once_in_the_final_seconds() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 1_000);
    chart.record_hud_activity(2_000, false);

    assert!(!chart.hud_safety_warning_due(13_999));
    assert!(chart.hud_safety_warning_due(14_000));

    chart.mark_hud_idle_warning_sounded();
    assert!(!chart.hud_safety_warning_due(15_000));

    // Renewed activity re-arms the once-per-session warning.
    chart.record_hud_activity(21_000, false);
    assert!(chart.hud_safety_warning_due(33_500));
    // Once the timeout itself is due the warning no longer applies.
    assert!(!chart.hud_safety_warning_due(36_000));
}

#[test]
fn hud_idle_warning_stays_silent_while_hovering() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 1_000);

    assert!(!chart.hud_safety_warning_due(60_000));
}

#[test]
fn hud_limit_click_side_follows_the_market_reference_price() {
    let mut chart = CandlestickChart::new(1);
    assert_eq!(chart.hud_limit_click_is_buy(100.0), None);

    chart.set_market_reference_price(Some(100.0));
    // At or below the reference rests a bid; above rests an ask.
    assert_eq!(chart.hud_limit_click_is_buy(99.0), Some(true));
    assert_eq!(chart.hud_limit_click_is_buy(100.0), Some(true));
    assert_eq!(chart.hud_limit_click_is_buy(101.0), Some(false));
}

#[test]
fn hud_style_changes_clear_armed_state() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::Hud);
    chart.set_hud_armed_at(true, 1_000);

    chart.set_crosshair_style(ChartCrosshairStyle::RacingHud);

    assert!(!chart.hud_armed());
    assert!(!chart.hud_order_submission_enabled());
}
