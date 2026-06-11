use super::*;
use crate::helpers::{assert_close, assert_close_fine, assert_close_loose};

fn assert_points_close(actual: &[Point], expected: &[(f32, f32)]) {
    assert_eq!(actual.len(), expected.len());
    for (point, (x, y)) in actual.iter().zip(expected) {
        assert_close_loose(point.x, *x);
        assert_close_loose(point.y, *y);
    }
}

const BIDS: [(f64, f64, f64); 2] = [(99.0, 1.0, 1.0), (97.0, 2.0, 3.0)];
const ASKS: [(f64, f64, f64); 2] = [(101.0, 1.0, 1.0), (104.0, 2.0, 3.0)];

fn layout_under_test() -> DepthChartLayout {
    depth_chart_layout(&BIDS, &ASKS, 100.0, 1.0, 100.0, 80.0).expect("layout")
}

#[test]
fn layout_window_uses_smaller_side_extent_quantized_in_ticks() {
    let layout = layout_under_test();

    // Bid extent 3, ask extent 4: the smaller side (3 ticks) quantizes up to
    // the next 1-2-5 step (5 ticks).
    assert_close(layout.half_range, 5.0);
    assert_close(layout.price_min(), 95.0);
    assert_close(layout.price_max(), 105.0);
}

#[test]
fn layout_max_cum_is_quantized_to_a_nice_step() {
    let layout = layout_under_test();

    // Both sides reach cumulative 3 inside the window; quantized to 5.
    assert_close(layout.max_cum, 5.0);
}

#[test]
fn layout_requires_mid_tick_and_levels() {
    assert!(depth_chart_layout(&[], &[], 100.0, 1.0, 100.0, 80.0).is_none());
    assert!(depth_chart_layout(&BIDS, &ASKS, f64::NAN, 1.0, 100.0, 80.0).is_none());
    assert!(depth_chart_layout(&BIDS, &ASKS, 100.0, 0.0, 100.0, 80.0).is_none());
    assert!(depth_chart_layout(&BIDS, &ASKS, 100.0, 1.0, 0.0, 80.0).is_none());
}

#[test]
fn layout_with_one_empty_side_uses_the_other_extent() {
    let layout = depth_chart_layout(&BIDS, &[], 100.0, 1.0, 100.0, 80.0).expect("layout");

    // Bid extent 3 quantizes to 5 ticks.
    assert_close(layout.half_range, 5.0);
    assert_close(layout.max_cum, 5.0);
}

#[test]
fn price_x_mapping_centers_the_mid() {
    let layout = layout_under_test();

    assert_close_fine(layout.x_for_price(100.0), 50.0_f32);
    assert_close_fine(layout.x_for_price(95.0), 0.0_f32);
    assert_close_fine(layout.x_for_price(105.0), 100.0_f32);
    assert_close_fine(layout.price_at_x(75.0), 102.5);
}

#[test]
fn cum_y_mapping_spans_baseline_to_top_margin() {
    let layout = layout_under_test();

    assert_close_fine(layout.y_for_cum(0.0), 80.0_f32);
    // Max cumulative leaves the 10% top margin clear: 80 - 0.9 * 80 = 8.
    assert_close_fine(layout.y_for_cum(layout.max_cum), 8.0_f32);
}

#[test]
fn bid_side_points_step_outward_and_extend_to_the_left_edge() {
    let layout = layout_under_test();

    let points = side_points(&BIDS, &layout, true);

    let y0 = layout.y_for_cum(0.0);
    let y1 = layout.y_for_cum(1.0);
    let y3 = layout.y_for_cum(3.0);
    assert_points_close(
        &points,
        &[(40.0, y0), (40.0, y1), (20.0, y1), (20.0, y3), (0.0, y3)],
    );
}

#[test]
fn ask_side_points_step_outward_and_extend_to_the_right_edge() {
    let layout = layout_under_test();

    let points = side_points(&ASKS, &layout, false);

    let y0 = layout.y_for_cum(0.0);
    let y1 = layout.y_for_cum(1.0);
    let y3 = layout.y_for_cum(3.0);
    assert_points_close(
        &points,
        &[(60.0, y0), (60.0, y1), (90.0, y1), (90.0, y3), (100.0, y3)],
    );
}

#[test]
fn side_points_stop_at_the_price_window() {
    // Deep bid at 89 sits far outside the 2-tick window set by the asks.
    let bids = [(99.0, 1.0, 1.0), (98.0, 1.0, 2.0), (89.0, 5.0, 7.0)];
    let asks = [(101.0, 1.0, 1.0), (102.0, 1.0, 2.0)];
    let layout = depth_chart_layout(&bids, &asks, 100.0, 1.0, 100.0, 80.0).expect("layout");

    assert_close(layout.half_range, 2.0);
    let points = side_points(&bids, &layout, true);

    // Two in-window levels produce four step points; the deep level is not
    // walked and the last in-window level already sits on the left edge.
    assert_eq!(points.len(), 4);
    assert_close_fine(points[3].x, 0.0_f32);
    assert_close_fine(points[3].y, layout.y_for_cum(2.0));
}

#[test]
fn cum_at_price_accumulates_levels_at_or_better() {
    assert_eq!(cum_at_price(&BIDS, 98.0, true), Some(1.0));
    assert_eq!(cum_at_price(&BIDS, 99.0, true), Some(1.0));
    assert_eq!(cum_at_price(&BIDS, 97.0, true), Some(3.0));
    assert_eq!(cum_at_price(&BIDS, 96.0, true), Some(3.0));
    assert_eq!(cum_at_price(&BIDS, 99.5, true), None);

    assert_eq!(cum_at_price(&ASKS, 102.0, false), Some(1.0));
    assert_eq!(cum_at_price(&ASKS, 101.0, false), Some(1.0));
    assert_eq!(cum_at_price(&ASKS, 105.0, false), Some(3.0));
    assert_eq!(cum_at_price(&ASKS, 100.5, false), None);
}

#[test]
fn hover_target_buckets_the_cursor_price_per_side() {
    let layout = layout_under_test();

    // x=29 -> price 97.9, bid side, inward (ceil) bucket 98.
    let bid_target = hover_target(&BIDS, &ASKS, &layout, 29.0).expect("bid target");
    assert!(bid_target.is_bid);
    assert_close(bid_target.price, 98.0);
    assert_close(bid_target.cum, 1.0);

    // x=71 -> price 102.1, ask side, inward (floor) bucket 102.
    let ask_target = hover_target(&BIDS, &ASKS, &layout, 71.0).expect("ask target");
    assert!(!ask_target.is_bid);
    assert_close(ask_target.price, 102.0);
    assert_close(ask_target.cum, 1.0);
}

#[test]
fn hover_between_grid_lines_matches_the_painted_step() {
    let layout = layout_under_test();

    // x=25 -> price 97.5, between the risers at 97 and 99. The painted curve
    // there still sits at cumulative 1 (the riser to 3 happens at 97), so the
    // hover must report the inward bucket, not the next wall outward.
    let target = hover_target(&BIDS, &ASKS, &layout, 25.0).expect("target");
    assert!(target.is_bid);
    assert_close(target.price, 98.0);
    assert_close(target.cum, 1.0);
}

#[test]
fn hover_target_is_none_for_an_empty_side() {
    let layout = depth_chart_layout(&BIDS, &[], 100.0, 1.0, 100.0, 80.0).expect("layout");

    assert!(hover_target(&BIDS, &[], &layout, 75.0).is_none());
}

#[test]
fn axis_size_labels_are_compact() {
    assert_eq!(axis_size_label(500.0), "500");
    assert_eq!(axis_size_label(1_250.0), "1.25K");
    assert_eq!(axis_size_label(5_000.0), "5K");
    assert_eq!(axis_size_label(2_500_000.0), "2.5M");
    assert_eq!(axis_size_label(0.5), "0.5");
    assert_eq!(axis_size_label(0.0125), "0.0125");
    assert_eq!(axis_size_label(0.0), "0");
}

#[test]
fn layout_window_is_clamped_at_zero() {
    // Deep coverage on a low-priced market: 205 ticks of bid extent quantize
    // to 500 ticks (0.05), past the 0.021 mid. The window must stop at zero.
    let bids = [(0.0209, 1.0, 1.0), (0.0005, 1.0, 2.0)];
    let asks = [(0.0211, 1.0, 1.0), (0.05, 1.0, 2.0)];
    let layout = depth_chart_layout(&bids, &asks, 0.021, 0.0001, 100.0, 80.0).expect("layout");

    assert_close(layout.half_range, 0.021);
    assert_close(layout.price_min(), 0.0);
}

#[test]
fn hover_target_rejects_non_positive_prices() {
    let bids = [(0.0209, 1.0, 1.0), (0.0005, 1.0, 2.0)];
    let asks = [(0.0211, 1.0, 1.0), (0.05, 1.0, 2.0)];
    let layout = depth_chart_layout(&bids, &asks, 0.021, 0.0001, 100.0, 80.0).expect("layout");

    // The left edge of the zero-clamped window maps to price 0.
    assert!(hover_target(&bids, &asks, &layout, 0.0).is_none());
}

#[test]
fn layout_window_keeps_both_best_buckets_visible() {
    // Floor/ceil aggregation puts the best bid bucket 1.1 ticks below the
    // mid while the ask extent is only 0.9 ticks; sizing the window from the
    // smaller extent alone would cut the entire bid side.
    let bids = [(99.0, 1.0, 1.0)];
    let asks = [(101.0, 1.0, 1.0)];
    let layout = depth_chart_layout(&bids, &asks, 100.1, 1.0, 100.0, 80.0).expect("layout");

    assert_close(layout.half_range, 2.0);
    assert!(!side_points(&bids, &layout, true).is_empty());
    assert!(!side_points(&asks, &layout, false).is_empty());
}

#[test]
fn max_cum_quantizes_fractional_maxima() {
    // Sub-unit cumulative depth must keep a data-relative scale instead of
    // flooring at 1.0 and squashing the curve to the baseline.
    let bids = [(99.0, 0.1, 0.1), (97.0, 0.2, 0.3)];
    let asks = [(101.0, 0.1, 0.1)];
    let layout = depth_chart_layout(&bids, &asks, 100.0, 1.0, 100.0, 80.0).expect("layout");

    assert_close(layout.max_cum, 0.1);
}

#[test]
fn fractional_nice_step_ceil_extends_below_one() {
    assert_close(nice_step_ceil_fractional(0.05), 0.05);
    assert_close(nice_step_ceil_fractional(0.037), 0.05);
    assert_close(nice_step_ceil_fractional(0.3), 0.5);
    assert_close(nice_step_ceil_fractional(3.0), 5.0);
}

#[test]
fn marker_xs_filters_to_the_window() {
    let layout = layout_under_test();

    let xs = marker_xs(&[96.0, 100.0, 106.0, 94.0], &layout);

    assert_eq!(xs.len(), 2);
    assert_close_loose(xs[0], 10.0_f32);
    assert_close_loose(xs[1], 50.0_f32);
}
