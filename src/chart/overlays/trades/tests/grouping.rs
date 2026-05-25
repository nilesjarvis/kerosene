use super::*;

#[test]
fn groups_multiple_fills_on_the_same_candle_and_side() {
    let candles = vec![candle(0), candle(60_000), candle(120_000)];
    let markers = vec![
        marker(5_000, 100.0, 1.0, true),
        marker(10_000, 110.0, 3.0, true),
        marker(70_000, 120.0, 1.0, false),
    ];

    let groups = visible_trade_marker_groups(&candles, &markers, 0, 2);

    assert_eq!(groups.len(), 2);
    let buy = marker_group_for_side_or_panic(&groups, true);
    assert_eq!(buy.candle_idx, 0);
    assert_eq!(buy.count, 2);
    assert!((buy.price - 107.5).abs() < f64::EPSILON);

    let sell = marker_group_for_side_or_panic(&groups, false);
    assert_eq!(sell.candle_idx, 1);
    assert_eq!(sell.count, 1);
}

#[test]
fn coarsens_dense_visible_history_into_limited_groups() {
    let candles: Vec<_> = (0..400).map(|idx| candle(idx * 60_000)).collect();
    let markers: Vec<_> = candles
        .iter()
        .map(|candle| marker(candle.open_time + 1_000, 100.0, 1.0, true))
        .collect();

    let groups = visible_trade_marker_groups(&candles, &markers, 0, candles.len() - 1);

    assert!(groups.len() < markers.len());
    assert!(groups.len() <= TRADE_MARKER_MAX_GROUPS);
    assert_eq!(groups.iter().map(|group| group.count).sum::<usize>(), 400);
}

#[test]
fn skips_markers_outside_visible_candles_or_invalid_prices() {
    let candles = vec![candle(60_000), candle(120_000)];
    let markers = vec![
        marker(10_000, 100.0, 1.0, true),
        marker(65_000, -1.0, 1.0, true),
        marker(125_000, 101.0, 1.0, true),
        marker(240_000, 100.0, 1.0, false),
    ];

    let groups = visible_trade_marker_groups(&candles, &markers, 0, 1);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].candle_idx, 1);
    assert_eq!(groups[0].count, 1);
}
