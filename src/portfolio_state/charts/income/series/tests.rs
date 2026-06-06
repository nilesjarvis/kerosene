use iced::Point;

use crate::helpers::assert_close_loose as assert_near;

use super::*;

fn bars() -> Vec<(String, f64)> {
    vec![("Jan".to_string(), -10.0), ("Feb".to_string(), 10.0)]
}

#[test]
fn layout_rejects_empty_bars() {
    assert!(prepare_income_chart_layout(&[], 120.0, 80.0).is_none());
}

#[test]
fn layout_maps_negative_and_positive_bars_around_zero() {
    let layout = prepare_income_chart_layout(&bars(), 120.0, 80.0).unwrap();

    assert_near(layout.plot_width, 96.0);
    assert_near(layout.plot_height, 38.0);
    assert_near(layout.zero_y, 37.0);
    assert_near(layout.group_width, 48.0);
    assert_near(layout.bars[0].center_x, 36.0);
    assert_near(layout.bars[0].y, 37.0);
    assert_near(layout.bars[0].height, 15.96);
    assert_near(layout.bars[1].center_x, 84.0);
    assert_near(layout.bars[1].y, 21.04);
    assert_near(layout.bars[1].height, 15.96);
}

#[test]
fn layout_keeps_zero_bars_visible() {
    let zero_bars = vec![("Jan".to_string(), 0.0)];
    let layout = prepare_income_chart_layout(&zero_bars, 120.0, 80.0).unwrap();

    assert_near(layout.bars[0].y, layout.zero_y);
    assert_near(layout.bars[0].height, 1.0);
}

#[test]
fn hover_selects_bar_by_group_and_rejects_outside_plot() {
    let layout = prepare_income_chart_layout(&bars(), 120.0, 80.0).unwrap();

    assert_eq!(
        hovered_income_bar(&layout, Point::new(20.0, 30.0))
            .unwrap()
            .label,
        "Jan"
    );
    assert_eq!(
        hovered_income_bar(&layout, Point::new(80.0, 30.0))
            .unwrap()
            .label,
        "Feb"
    );
    assert!(hovered_income_bar(&layout, Point::new(8.0, 30.0)).is_none());
    assert!(hovered_income_bar(&layout, Point::new(20.0, 8.0)).is_none());
}

#[test]
fn tooltip_width_and_position_are_clamped_to_bounds() {
    let layout = prepare_income_chart_layout(&bars(), 120.0, 80.0).unwrap();
    let tooltip = income_tooltip_layout(&layout.bars[1], "+$10.00", 120.0, 80.0);

    assert_near(tooltip.width, 132.0);
    assert_near(tooltip.height, 38.0);
    assert_near(tooltip.origin.x, 4.0);
    assert_near(tooltip.origin.y, 4.0);
}
