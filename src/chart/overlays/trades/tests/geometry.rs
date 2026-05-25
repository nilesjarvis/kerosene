use super::*;

#[test]
fn marker_clamp_bounds_reject_tiny_price_areas() {
    assert!(trade_marker_clamp_bounds(TRADE_MARKER_MIN_PRICE_HEIGHT - 0.1).is_none());

    let (min_y, max_y) = marker_clamp_bounds_or_panic(TRADE_MARKER_MIN_PRICE_HEIGHT);
    assert!(min_y <= max_y);
}

#[test]
fn grouped_fill_dots_use_uniform_radius() {
    assert_eq!(trade_marker_dot_radius(4, 4, 3), TRADE_MARKER_RADIUS);
    assert_eq!(trade_marker_dot_radius(5, 4, 0), TRADE_MARKER_RADIUS);
    assert_eq!(trade_marker_dot_radius(5, 4, 3), TRADE_MARKER_RADIUS);
}

#[test]
fn marker_anchor_keeps_dots_away_from_candle_edges() {
    let candle = candle(0);
    let price_to_y = |price: f64| (120.0 - price) as f32;

    let buy_y = marker_anchor_or_panic(&candle, true, &price_to_y);
    let sell_y = marker_anchor_or_panic(&candle, false, &price_to_y);

    assert!(buy_y > price_to_y(candle.low) + TRADE_MARKER_CANDLE_GAP);
    assert!(sell_y < price_to_y(candle.high) - TRADE_MARKER_CANDLE_GAP);
}

#[test]
fn marker_anchor_uses_visual_edges_when_axis_is_inverted() {
    let candle = candle(0);
    let inverted_price_to_y = |price: f64| price as f32;

    let buy_y = marker_anchor_or_panic(&candle, true, &inverted_price_to_y);
    let sell_y = marker_anchor_or_panic(&candle, false, &inverted_price_to_y);

    assert!(buy_y > inverted_price_to_y(candle.high) + TRADE_MARKER_CANDLE_GAP);
    assert!(sell_y < inverted_price_to_y(candle.low) - TRADE_MARKER_CANDLE_GAP);
}
