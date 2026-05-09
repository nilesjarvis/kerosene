use iced::Point;

use super::*;

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn layout_rejects_too_few_points_or_flat_time_range() {
    assert!(prepare_pnl_chart_layout(&[(1_000, 10.0)], 120.0, 80.0).is_none());
    assert!(prepare_pnl_chart_layout(&[(1_000, 10.0), (1_000, 12.0)], 120.0, 80.0).is_none());
    assert!(prepare_pnl_chart_layout(&[(1_000, 10.0), (2_000, 12.0)], 0.0, 80.0).is_none());
}

#[test]
fn layout_maps_time_edges_and_pads_pnl_range() {
    let layout = prepare_pnl_chart_layout(&[(1_000, -10.0), (2_000, 10.0)], 100.0, 50.0).unwrap();

    assert_near(layout.points[0].point.x, 0.0);
    assert_near(layout.points[1].point.x, 100.0);
    assert_near(layout.points[0].point.y, 46.5517);
    assert_near(layout.points[1].point.y, 3.4483);
    assert_near(layout.zero_y, 25.0);
}

#[test]
fn layout_expands_flat_pnl_values() {
    let layout = prepare_pnl_chart_layout(&[(1_000, 5.0), (2_000, 5.0)], 100.0, 50.0).unwrap();

    assert_near(layout.points[0].point.y, 25.0);
    assert_near(layout.points[1].point.y, 25.0);
    assert_near(layout.zero_y, 50.0);
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
