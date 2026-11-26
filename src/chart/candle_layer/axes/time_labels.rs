use crate::app_time::DisplayTimeZone;
use crate::timeframe::Timeframe;

// ---------------------------------------------------------------------------
// Time Axis Labels
// ---------------------------------------------------------------------------

const MONTH_AXIS_SPAN_SECS: u64 = 90 * 24 * 60 * 60;
const YEAR_AXIS_SPAN_SECS: u64 = 366 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TimeAxisLabelMode {
    Time,
    DateTime,
    Month,
    MonthYear,
}

impl TimeAxisLabelMode {
    pub(super) fn for_timeframe_and_span(timeframe: Timeframe, span_secs: u64) -> Self {
        if span_secs >= YEAR_AXIS_SPAN_SECS {
            Self::MonthYear
        } else if span_secs >= MONTH_AXIS_SPAN_SECS {
            Self::Month
        } else if uses_time_only_axis(timeframe) {
            Self::Time
        } else {
            Self::DateTime
        }
    }
}

pub(super) fn format_time_axis_label(
    zone: DisplayTimeZone,
    unix_ms: u64,
    mode: TimeAxisLabelMode,
) -> String {
    let pattern = match mode {
        TimeAxisLabelMode::Time => "%H:%M",
        TimeAxisLabelMode::DateTime => "%m/%d %H:%M",
        TimeAxisLabelMode::Month => "%b",
        TimeAxisLabelMode::MonthYear => "%b %y",
    };
    zone.format(unix_ms, pattern, "--")
}

fn uses_time_only_axis(timeframe: Timeframe) -> bool {
    timeframe.duration_ms() <= Timeframe::H1.duration_ms()
}
