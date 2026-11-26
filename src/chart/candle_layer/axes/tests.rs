use super::{TimeAxisLabelMode, format_time_axis_label};
use crate::timeframe::Timeframe;

#[test]
fn low_timeframe_axis_labels_use_time_only() {
    assert_eq!(
        format_time_axis_label(
            Default::default(),
            1_714_566_840_000,
            TimeAxisLabelMode::Time
        ),
        "12:34"
    );
}

#[test]
fn monthly_axis_labels_use_month_names() {
    assert_eq!(
        format_time_axis_label(
            Default::default(),
            1_714_521_600_000,
            TimeAxisLabelMode::Month
        ),
        "May"
    );
    assert_eq!(
        format_time_axis_label(
            Default::default(),
            1_714_521_600_000,
            TimeAxisLabelMode::MonthYear
        ),
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

#[test]
fn timezone_labels_follow_dst_fractional_offsets_and_calendar_rollovers() {
    use crate::app_time::DisplayTimeZone;
    let ny = DisplayTimeZone(chrono_tz::America::New_York);
    let kathmandu = DisplayTimeZone(chrono_tz::Asia::Kathmandu);
    let label = |zone, stamp: &str, mode| {
        let utc: chrono::DateTime<chrono::Utc> = stamp.parse().expect("valid instant");
        format_time_axis_label(zone, utc.timestamp_millis() as u64, mode)
    };
    assert_eq!(
        label(ny, "2026-03-08T06:59:00Z", TimeAxisLabelMode::Time),
        "01:59"
    );
    assert_eq!(
        label(ny, "2026-03-08T07:00:00Z", TimeAxisLabelMode::Time),
        "03:00"
    );
    assert_eq!(
        label(ny, "2026-01-01T03:30:00Z", TimeAxisLabelMode::MonthYear),
        "Dec 25"
    );
    assert_eq!(
        label(
            kathmandu,
            "2026-01-01T20:00:00Z",
            TimeAxisLabelMode::DateTime
        ),
        "01/02 01:45"
    );
    assert_eq!(
        format_time_axis_label(ny, u64::MAX, TimeAxisLabelMode::DateTime),
        "--"
    );
}
