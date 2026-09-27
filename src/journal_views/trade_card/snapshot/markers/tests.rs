use super::*;

#[test]
fn marker_groups_chain_adjacent_fills_without_combining_sides() {
    let plot = SnapshotPlot {
        left: 0.0,
        top: 50.0,
        width: 100.0,
        height: 100.0,
        start_ms: 0,
        end_ms: 1_000,
        min_price: 1.0,
        max_price: 2.0,
    };
    let markers = [
        (280, true),
        (100, false),
        (190, true),
        (100, true),
        (700, true),
        (190, false),
    ]
    .map(|(time_ms, is_buy)| TradeMarker {
        time_ms,
        is_buy,
        price: 1.0,
        size: 1.0,
    });
    let layouts = marker_group_layouts(plot, &markers);
    assert_eq!(layouts.len(), 3);
    for (layout, (x, is_buy, radius)) in layouts.iter().zip([
        (14.5, false, SNAPSHOT_MARKER_GROUP_RADIUS),
        (19.0, true, SNAPSHOT_MARKER_GROUP_RADIUS),
        (70.0, true, SNAPSHOT_MARKER_RADIUS),
    ]) {
        assert!((layout.center.x - x).abs() < 1e-5);
        assert_eq!(layout.is_buy, is_buy);
        assert_eq!(layout.radius, radius);
    }
    assert!(marker_group_layouts(plot, &[]).is_empty());

    let mut boundary = Vec::new();
    push_marker_side_layouts(
        plot,
        &[0.0, SNAPSHOT_MARKER_GROUP_DISTANCE],
        true,
        &mut boundary,
    );
    assert_eq!(boundary.len(), 1);
    assert_eq!(boundary[0].radius, SNAPSHOT_MARKER_GROUP_RADIUS);
}

#[test]
fn marker_group_layouts_collapse_nearby_fills() {
    let plot = SnapshotPlot {
        left: 0.0,
        top: 50.0,
        width: 100.0,
        height: 100.0,
        start_ms: 0,
        end_ms: 1_000,
        min_price: 1.0,
        max_price: 2.0,
    };
    let markers = vec![
        TradeMarker {
            time_ms: 100,
            price: 1.0,
            size: 1.0,
            is_buy: true,
        },
        TradeMarker {
            time_ms: 105,
            price: 1.0,
            size: 1.0,
            is_buy: true,
        },
        TradeMarker {
            time_ms: 350,
            price: 1.0,
            size: 1.0,
            is_buy: true,
        },
    ];

    let layouts = marker_group_layouts(plot, &markers);

    assert_eq!(layouts.len(), 2);
    assert!(layouts.iter().all(|layout| layout.is_buy));
    assert_eq!(layouts[0].radius, SNAPSHOT_MARKER_GROUP_RADIUS);
    assert!(layouts[1].center.x - layouts[0].center.x > layouts[0].radius + layouts[1].radius);
}

#[test]
fn marker_group_layouts_place_sells_above_and_buys_below_plot() {
    let plot = SnapshotPlot {
        left: 0.0,
        top: 50.0,
        width: 100.0,
        height: 100.0,
        start_ms: 0,
        end_ms: 1_000,
        min_price: 1.0,
        max_price: 2.0,
    };
    let markers = vec![
        TradeMarker {
            time_ms: 100,
            price: 1.0,
            size: 1.0,
            is_buy: false,
        },
        TradeMarker {
            time_ms: 900,
            price: 1.0,
            size: 1.0,
            is_buy: true,
        },
    ];

    let layouts = marker_group_layouts(plot, &markers);
    let sell = layouts
        .iter()
        .find(|layout| !layout.is_buy)
        .expect("sell marker");
    let buy = layouts
        .iter()
        .find(|layout| layout.is_buy)
        .expect("buy marker");

    assert_eq!(sell.radius, SNAPSHOT_MARKER_RADIUS);
    assert_eq!(buy.radius, SNAPSHOT_MARKER_RADIUS);
    assert!(sell.center.y + sell.radius <= plot.top - SNAPSHOT_MARKER_CHART_GAP);
    assert!(buy.center.y - buy.radius >= plot.top + plot.height + SNAPSHOT_MARKER_CHART_GAP);
}
