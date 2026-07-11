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

#[test]
fn income_layout_debug_redacts_account_values_and_derived_geometry() {
    let bar = IncomeBarLayout {
        label: "private-income-label-sentinel".to_string(),
        value: 98_765.432_1,
        center_x: 11.123_45,
        x: 22.234_56,
        y: 33.345_67,
        width: 44.456_78,
        height: 55.567_89,
        scaled: 66.678_91,
        show_axis_label: true,
    };
    let layout = IncomeChartLayout {
        bars: vec![bar.clone()],
        left_pad: 77.789_12,
        top_pad: 88.891_23,
        bottom_pad: 99.912_34,
        plot_width: 111.123_45,
        plot_height: 222.234_56,
        zero_y: 333.345_67,
        group_width: 444.456_8,
    };

    let rendered = format!("{bar:?} {layout:?}");

    assert!(rendered.contains("bars_count: 1"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(
        !rendered.contains("private-income-label-sentinel"),
        "{rendered}"
    );
    for value in [98_765.432_1_f64, 11.123_45, 55.567_89] {
        assert!(!rendered.contains(&format!("{value:?}")), "{rendered}");
    }
    assert_eq!(bar.label, "private-income-label-sentinel");
    assert_eq!(bar.value.to_bits(), 98_765.432_1_f64.to_bits());
    assert_eq!(layout.zero_y.to_bits(), 333.345_67_f32.to_bits());
}
