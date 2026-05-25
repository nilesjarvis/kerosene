use super::{TimeAxisLabelMode, format_time_axis_label};
use crate::timeframe::Timeframe;

#[test]
fn low_timeframe_axis_labels_use_time_only() {
    assert_eq!(
        format_time_axis_label(1_714_566_840, TimeAxisLabelMode::Time),
        "12:34"
    );
}

#[test]
fn monthly_axis_labels_use_month_names() {
    assert_eq!(
        format_time_axis_label(1_714_521_600, TimeAxisLabelMode::Month),
        "May"
    );
    assert_eq!(
        format_time_axis_label(1_714_521_600, TimeAxisLabelMode::MonthYear),
        "May 24"
    );
}

#[test]
fn axis_label_mode_switches_to_months_for_wide_views() {
    assert_eq!(
        TimeAxisLabelMode::for_timeframe_and_span(Timeframe::M15, 30 * 24 * 60 * 60),
        TimeAxisLabelMode::Time
    );
    assert_eq!(
        TimeAxisLabelMode::for_timeframe_and_span(Timeframe::H4, 30 * 24 * 60 * 60),
        TimeAxisLabelMode::DateTime
    );
    assert_eq!(
        TimeAxisLabelMode::for_timeframe_and_span(Timeframe::M15, 120 * 24 * 60 * 60),
        TimeAxisLabelMode::Month
    );
    assert_eq!(
        TimeAxisLabelMode::for_timeframe_and_span(Timeframe::M15, 400 * 24 * 60 * 60),
        TimeAxisLabelMode::MonthYear
    );
}
