use super::{
    HudReadoutLabels, format_crosshair_relative_time, format_volume_compact,
    hud_game_panels_visible, hud_readout_lines,
};
use crate::config::{ChartCrosshairStyle, ChartHudReadoutConfig};
use iced::Point;

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
fn hud_game_panels_require_hover_inside_chart_area() {
    assert!(hud_game_panels_visible(
        ChartCrosshairStyle::Hud,
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
fn hud_readout_lines_follow_visibility_config() {
    let config = ChartHudReadoutConfig {
        price: false,
        clock: false,
        candle_close: false,
        ..ChartHudReadoutConfig::default()
    };

    let (left, right) = hud_readout_lines(
        config,
        HudReadoutLabels {
            symbol: "HYPE",
            timeframe: "1H",
            hover_price: 42.0,
            data_pos: Point::new(12.0, 34.0),
            hover_time: "01/01 00:00:00",
            clock: "00:00:01",
            candle_close: "59m",
        },
    );

    assert_eq!(
        left,
        vec!["HYPE 1H".to_string(), "XY  12.0  34.0".to_string()]
    );
    assert_eq!(right, vec!["T  01/01 00:00:00".to_string()]);
}
