use super::*;
use iced::Size;

fn fill(time_ms: u64, price: f64, size: f64, is_buy: bool) -> TradeMarker {
    TradeMarker {
        time_ms,
        price,
        size,
        is_buy,
    }
}

#[test]
fn dots_follow_their_own_candle_and_group_only_its_same_side_fills() {
    let candles = [
        Candle::test_ohlcv(100, 199, [101.0, 104.0, 98.0, 102.0], 10.0),
        Candle::test_ohlcv(200, 299, [112.0, 115.0, 108.0, 110.0], 10.0),
    ];
    // Adjacent candles can project less than one dot-width apart in time.
    let plot = SnapshotPlot::new(Size::new(400.0, 240.0), 0, 20_000, &candles, []);
    let groups = marker_groups(
        plot,
        &candles,
        &[
            fill(105, 100.0, 1.0, true),
            fill(150, 101.0, 0.5, false),
            fill(199, 104.0, 3.0, true),
            fill(200, 112.0, 0.25, true),
        ],
    );
    assert_eq!(groups.len(), 3);
    let (sell, buys, next_buy) = (&groups[0], &groups[1], &groups[2]);
    assert!(!sell.is_buy);
    assert!(buys.is_buy && next_buy.is_buy);
    assert_eq!((buys.count, buys.size, buys.average_price), (2, 4.0, 103.0));
    assert_eq!((buys.first_ms, buys.last_ms), (105, 199));
    assert_eq!((sell.count, sell.size, sell.average_price), (1, 0.5, 101.0));
    assert_eq!(sell.center.x, plot.x_for_time(100));
    assert_eq!(
        sell.center.y,
        plot.y_for_price(104.0) - (DOT_RADIUS + WICK_GAP)
    );
    assert_eq!(buys.center.x, sell.center.x);
    assert_eq!(
        buys.center.y,
        plot.y_for_price(98.0) + GROUP_RADIUS + WICK_GAP
    );
    assert_eq!(next_buy.center.x, plot.x_for_time(200));
    assert_eq!(
        next_buy.center.y,
        plot.y_for_price(108.0) + (DOT_RADIUS + WICK_GAP)
    );
}

#[test]
fn nearest_drawn_dot_wins_with_a_small_hover_margin() {
    let candles = [Candle::test_flat(0, 100.0)];
    let plot = SnapshotPlot::new(Size::new(400.0, 240.0), 0, 60_000, &candles, []);
    let groups = marker_groups(
        plot,
        &candles,
        &[fill(100, 100.0, 1.0, false), fill(100, 100.0, 2.0, true)],
    );
    for (i, group) in groups.iter().enumerate() {
        let near = Point::new(group.center.x + DOT_RADIUS + 2.0, group.center.y);
        assert_eq!(hit_test(&groups, near), Some(i));
    }
    let x = groups[0].center.x;
    assert_eq!(
        hit_test(&groups, Point::new(x, plot.y_for_price(100.0) + 1.0)),
        Some(1)
    );
    assert_eq!(
        hit_test(&groups, Point::new(x, plot.top + plot.height)),
        None
    );
    assert_eq!(hit_test(&[], Point::ORIGIN), None);
}

#[test]
fn missing_candle_uses_the_execution_time_and_price() {
    let candles = [Candle::test_flat(100_000, 100.0)];
    let gap_fill = fill(99_999, 80.0, 1.0, true);
    let plot = SnapshotPlot::new(
        Size::new(400.0, 240.0),
        0,
        160_000,
        &candles,
        [gap_fill.price],
    );
    let groups = marker_groups(plot, &candles, &[gap_fill]);
    assert!(plot.min_price < gap_fill.price && plot.max_price > candles[0].high);
    let center = groups[0].center;
    assert_eq!(center.x, plot.x_for_time(gap_fill.time_ms));
    assert_eq!(
        center.y,
        plot.y_for_price(gap_fill.price) + (DOT_RADIUS + WICK_GAP)
    );
    assert!(center.y + DOT_RADIUS <= plot.top + plot.height);
}
