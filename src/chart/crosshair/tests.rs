use super::{
    HUD_JET_TAPE_GAP, format_crosshair_relative_time, format_volume_compact, format_volume_readout,
    hud_game_panels_visible, hud_jet_tape_side, hud_left_block_lines, hud_text_width,
};
use crate::api::Candle;
use crate::chart::crosshair_style::RacingHudMetrics;
use crate::chart::state::{HudMarketSide, HudOrderKind};
use crate::chart::{CandlestickChart, ChartState, EarningsMarker};
use crate::config::{ChartCrosshairStyle, ChartHudReadoutConfig};
use iced::Point;

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle::test_ohlcv(
        open_time,
        open_time + 999,
        [close, close + 1.0, close - 1.0, close],
        10.0,
    )
}

#[test]
fn format_crosshair_relative_time_handles_past_values() {
    let now_ms = 1_000_000_000;

    assert_eq!(
        format_crosshair_relative_time(now_ms - 10_000, now_ms),
        "10 seconds ago"
    );
    assert_eq!(
        format_crosshair_relative_time(now_ms - 60_000, now_ms),
        "1 minute ago"
    );
    assert_eq!(
        format_crosshair_relative_time(now_ms - 10 * 86_400_000, now_ms),
        "10 days ago"
    );
}

#[test]
fn format_crosshair_relative_time_handles_future_values() {
    let now_ms = 1_000_000_000;

    assert_eq!(
        format_crosshair_relative_time(now_ms + 3_600_000, now_ms),
        "in 1 hour"
    );
    assert_eq!(
        format_crosshair_relative_time(now_ms + 14 * 86_400_000, now_ms),
        "in 2 weeks"
    );
}

#[test]
fn format_crosshair_relative_time_treats_nearby_values_as_now() {
    let now_ms = 1_000_000_000;

    assert_eq!(format_crosshair_relative_time(now_ms, now_ms), "now");
    assert_eq!(
        format_crosshair_relative_time(now_ms.saturating_sub(4_999), now_ms),
        "now"
    );
}

#[test]
fn format_volume_compact_handles_zero_and_invalid_inputs() {
    assert_eq!(format_volume_compact(0.0), "0");
    assert_eq!(format_volume_compact(-12.5), "0");
    assert_eq!(format_volume_compact(f64::NAN), "0");
    assert_eq!(format_volume_compact(f64::INFINITY), "0");
}

#[test]
fn format_volume_compact_keeps_sub_unit_volumes_readable() {
    assert_eq!(format_volume_compact(0.0125), "0.0125");
}

#[test]
fn format_volume_compact_uses_two_decimals_below_a_thousand() {
    assert_eq!(format_volume_compact(5.5), "5.50");
    assert_eq!(format_volume_compact(999.99), "999.99");
}

#[test]
fn format_volume_compact_groups_with_k_m_and_b_suffixes() {
    assert_eq!(format_volume_compact(12_345.0), "12.3K");
    assert_eq!(format_volume_compact(5_000_000.0), "5.00M");
    assert_eq!(format_volume_compact(2_500_000_000.0), "2.50B");
}

#[test]
fn format_volume_readout_uses_whole_counts_for_whole_unit_markets() {
    assert_eq!(format_volume_readout(42.0, true), "42");
    assert_eq!(format_volume_readout(0.0, true), "0");
    assert_eq!(format_volume_readout(12_345.0, true), "12.3K");
}

#[test]
fn format_volume_readout_keeps_fractional_form_for_other_markets() {
    assert_eq!(format_volume_readout(5.5, false), "5.50");
    assert_eq!(format_volume_readout(0.0125, false), "0.0125");
}

#[test]
fn hud_game_panels_require_hover_inside_chart_area() {
    assert!(hud_game_panels_visible(
        ChartCrosshairStyle::Hud,
        Some(Point::new(120.0, 80.0)),
        300.0,
        200.0
    ));
    assert!(hud_game_panels_visible(
        ChartCrosshairStyle::RacingHud,
        Some(Point::new(120.0, 80.0)),
        300.0,
        200.0
    ));
    assert!(!hud_game_panels_visible(
        ChartCrosshairStyle::Hud,
        None,
        300.0,
        200.0
    ));
    assert!(!hud_game_panels_visible(
        ChartCrosshairStyle::Hud,
        Some(Point::new(320.0, 80.0)),
        300.0,
        200.0
    ));
    assert!(!hud_game_panels_visible(
        ChartCrosshairStyle::Hud,
        Some(Point::new(120.0, 220.0)),
        300.0,
        200.0
    ));
    assert!(!hud_game_panels_visible(
        ChartCrosshairStyle::Classic,
        Some(Point::new(120.0, 80.0)),
        300.0,
        200.0
    ));
}

#[test]
fn earnings_marker_hover_overlay_is_active_only_for_visible_hovered_markers() {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(vec![candle_at(1_000, 100.0), candle_at(2_000, 101.0)]);
    chart.set_earnings_markers(vec![EarningsMarker {
        time_ms: 2_000,
        cik: 1_652_044,
        form: "8-K".to_string(),
        filing_date: "2026-04-29".to_string(),
        accession_number: "0001652044-26-000043".to_string(),
        primary_document: "goog-20260429.htm".to_string(),
        quarter_label: Some("Q1 2026".to_string()),
        filing_summary: None,
        filing_summary_status: None,
        filing_summary_loading: false,
    }]);
    let state = ChartState::default();

    assert!(!chart.earnings_marker_hover_overlay_active(&state, 400.0));

    chart.set_earnings_marker_hover(Some(2_000));
    assert!(chart.earnings_marker_hover_overlay_active(&state, 400.0));

    chart.set_earnings_marker_hover(Some(3_000));
    assert!(!chart.earnings_marker_hover_overlay_active(&state, 400.0));
    assert!(!chart.earnings_marker_hover_overlay_active(&state, 0.0));
}

#[test]
fn hud_left_block_lines_follow_visibility_config() {
    let lines = hud_left_block_lines(
        ChartHudReadoutConfig::default(),
        "HYPE",
        "1H",
        Point::new(12.0, 34.0),
    );
    assert_eq!(
        lines,
        vec!["HYPE 1H".to_string(), "XY  12.0  34.0".to_string()]
    );

    let symbol_only = hud_left_block_lines(
        ChartHudReadoutConfig {
            coordinates: false,
            ..ChartHudReadoutConfig::default()
        },
        "HYPE",
        "1H",
        Point::new(12.0, 34.0),
    );
    assert_eq!(symbol_only, vec!["HYPE 1H".to_string()]);

    let none = hud_left_block_lines(
        ChartHudReadoutConfig {
            symbol: false,
            coordinates: false,
            ..ChartHudReadoutConfig::default()
        },
        "HYPE",
        "1H",
        Point::new(12.0, 34.0),
    );
    assert!(none.is_empty());
}

#[test]
fn jet_tape_side_prefers_its_slot_then_flips_then_gives_up() {
    let label_width = hud_text_width("64,213.5", 11.0);

    // Plenty of room: the preferred side wins.
    assert_eq!(hud_jet_tape_side(400.0, label_width, 1.0, 800.0), Some(1.0));
    assert_eq!(
        hud_jet_tape_side(400.0, label_width, -1.0, 800.0),
        Some(-1.0)
    );

    // Near the right edge the price tape flips left.
    assert_eq!(
        hud_jet_tape_side(780.0, label_width, 1.0, 800.0),
        Some(-1.0)
    );
    // Near the left edge the time tape flips right.
    assert_eq!(hud_jet_tape_side(20.0, label_width, -1.0, 800.0), Some(1.0));

    // On a chart too narrow for the full extent on either side, no tape.
    let wide_label = hud_text_width("06/10 13:45:12", 11.0);
    let extent = HUD_JET_TAPE_GAP + 6.0 + wide_label;
    assert!(extent * 2.0 > 200.0, "test premise: neither side fits");
    assert_eq!(hud_jet_tape_side(100.0, wide_label, -1.0, 200.0), None);
}

#[test]
fn racing_hud_metrics_use_current_size_relative_to_max_size() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::RacingHud);
    chart.set_market_reference_price(Some(50.0));
    chart.set_hud_max_notional(Some(1_000.0));
    let state = ChartState {
        hud_order_kind: HudOrderKind::Limit,
        hud_market_side: HudMarketSide::Long,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    assert_eq!(
        chart.racing_hud_metrics(&state, Some(100.0)),
        Some(RacingHudMetrics::new(Some(2.5), Some(20.0), None, None))
    );
}

#[test]
fn racing_hud_metrics_use_latest_candle_before_hover_price() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::RacingHud);
    chart.set_hud_max_notional(Some(1_000.0));
    chart.set_candles(vec![candle_at(1_000, 25.0)]);
    let state = ChartState {
        hud_order_kind: HudOrderKind::Limit,
        hud_market_side: HudMarketSide::Long,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    assert_eq!(
        chart.racing_hud_metrics(&state, Some(100.0)),
        Some(RacingHudMetrics::new(Some(2.5), Some(40.0), None, None))
    );
}

#[test]
fn racing_hud_metrics_include_current_spread() {
    let mut chart = CandlestickChart::new(1);
    chart.set_crosshair_style(ChartCrosshairStyle::RacingHud);
    chart.set_market_reference_price(Some(50.0));
    chart.set_current_spread_at(Some(0.025), 1_000);
    let state = ChartState {
        hud_order_kind: HudOrderKind::Limit,
        hud_market_side: HudMarketSide::Long,
        hud_size_input: "2.5".to_string(),
        ..ChartState::default()
    };

    assert_eq!(
        chart.racing_hud_metrics(&state, Some(100.0)),
        Some(RacingHudMetrics::new(
            Some(2.5),
            None,
            Some(0.025),
            Some((0.025, 0.025))
        ))
    );
}
