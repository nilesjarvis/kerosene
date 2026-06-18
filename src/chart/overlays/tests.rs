use super::*;
use crate::config::ChartSeriesStyle;
use iced::{Color, Theme};

#[test]
fn position_and_order_overlay_render_guard_follows_privacy_flag() {
    let mut chart = CandlestickChart::new(1);
    assert!(chart.should_draw_position_and_order_overlays());

    chart.hide_positions_and_orders = true;
    assert!(!chart.should_draw_position_and_order_overlays());
}

#[test]
fn current_price_badge_uses_line_color_in_line_mode() {
    let mut chart = CandlestickChart::new(1);
    chart.series_style = ChartSeriesStyle::Line;
    chart.chart_line_color = Some(Color::from_rgb8(0x9a, 0xd7, 0xff));
    let bull = Color::from_rgb8(0x00, 0xff, 0x88);
    let bear = Color::from_rgb8(0xff, 0x44, 0x66);

    let (badge_color, text_color) =
        current_price::current_price_badge_colors(&chart, false, &Theme::Dark, bull, bear);

    let expected_badge_color = Color::from_rgb8(0x9a, 0xd7, 0xff);
    assert_eq!(badge_color, expected_badge_color);
    assert_eq!(
        text_color,
        crate::helpers::text_color_for_bg(expected_badge_color)
    );
}

#[test]
fn current_price_badge_uses_candle_direction_in_candle_mode() {
    let chart = CandlestickChart::new(1);
    let bull = Color::from_rgb8(0x00, 0xff, 0x88);
    let bear = Color::from_rgb8(0xff, 0x44, 0x66);

    let (bull_badge, bull_text) =
        current_price::current_price_badge_colors(&chart, true, &Theme::Dark, bull, bear);
    let (bear_badge, bear_text) =
        current_price::current_price_badge_colors(&chart, false, &Theme::Dark, bull, bear);

    assert_eq!(bull_badge, bull);
    assert_eq!(bear_badge, bear);
    assert_eq!(bull_text, Color::BLACK);
    assert_eq!(bear_text, Color::BLACK);
}
