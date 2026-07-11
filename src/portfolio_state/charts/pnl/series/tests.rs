use iced::Point;

use crate::helpers::assert_close_loose as assert_near;

use super::*;

#[test]
fn layout_rejects_too_few_points_or_flat_time_range() {
    assert!(prepare_pnl_chart_layout(&[(1_000, 10.0)], 120.0, 80.0).is_none());
    assert!(prepare_pnl_chart_layout(&[(1_000, 10.0), (1_000, 12.0)], 120.0, 80.0).is_none());
    assert!(prepare_pnl_chart_layout(&[(1_000, 10.0), (2_000, 12.0)], 0.0, 80.0).is_none());
}

#[test]
fn layout_maps_time_edges_and_pads_pnl_range() {
    // Range [-10, 10]: 12% headroom below (vMin = -12.4), 16% above (vMax = 13.2).
    let layout = prepare_pnl_chart_layout(&[(1_000, -10.0), (2_000, 10.0)], 100.0, 50.0).unwrap();

    assert_near(layout.points[0].point.x, 0.0);
    assert_near(layout.points[1].point.x, 100.0);
    assert_near(layout.points[0].point.y, 45.3125);
    assert_near(layout.points[1].point.y, 6.25);
    assert_near(layout.zero_y, 25.78125);
}

#[test]
fn layout_keeps_zero_baseline_for_all_positive_series() {
    // A purely positive series still pins the zero baseline inside the plot.
    let layout = prepare_pnl_chart_layout(&[(1_000, 5.0), (2_000, 5.0)], 100.0, 50.0).unwrap();

    assert_near(layout.points[0].point.y, 6.25);
    assert_near(layout.points[1].point.y, 6.25);
    assert_near(layout.zero_y, 45.3125);
}

#[test]
fn layout_expands_all_zero_series() {
    let layout = prepare_pnl_chart_layout(&[(1_000, 0.0), (2_000, 0.0)], 100.0, 50.0).unwrap();

    assert_near(layout.points[0].point.y, 25.78125);
    assert_near(layout.points[1].point.y, 25.78125);
    assert_near(layout.zero_y, 25.78125);
}

#[test]
fn nearest_point_uses_horizontal_distance() {
    let layout =
        prepare_pnl_chart_layout(&[(1_000, -1.0), (2_000, 1.0), (3_000, 0.0)], 100.0, 50.0)
            .unwrap();

    let nearest = nearest_pnl_point(&layout.points, 54.0).unwrap();

    assert_eq!(nearest.timestamp_ms, 2_000);
}

#[test]
fn tooltip_origin_flips_left_and_clamps_vertically() {
    let origin = pnl_tooltip_origin(Point::new(195.0, 10.0), 220.0, 80.0);

    assert_near(origin.x, 4.0);
    assert_near(origin.y, 4.0);
}

#[test]
fn pnl_layout_debug_redacts_account_values_and_derived_geometry() {
    let point = PnlChartPoint {
        point: Point::new(123.456_7, 234.567_8),
        timestamp_ms: 9_876_543_210,
        pnl: -98_765.432_1,
    };
    let layout = PnlChartLayout {
        points: vec![point],
        zero_y: 345.678_9,
    };

    let rendered = format!("{point:?} {layout:?}");

    assert!(rendered.contains("points_count: 1"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    for sensitive in [
        "123.4567",
        "234.5678",
        "9876543210",
        "-98765.4321",
        "345.6789",
    ] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert_eq!(point.timestamp_ms, 9_876_543_210);
    assert_eq!(point.pnl.to_bits(), (-98_765.432_1_f64).to_bits());
    assert_eq!(layout.zero_y.to_bits(), 345.678_9_f32.to_bits());
}
