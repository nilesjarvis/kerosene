use crate::helpers::format_timestamp;
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

pub(super) fn format_time_axis_label(unix_secs: u64, mode: TimeAxisLabelMode) -> String {
    match mode {
        TimeAxisLabelMode::Time => format_time_of_day(unix_secs),
        TimeAxisLabelMode::DateTime => format_timestamp(unix_secs),
        TimeAxisLabelMode::Month => {
            let (_, month, _, _) = timestamp_parts(unix_secs);
            month_name(month).to_string()
        }
        TimeAxisLabelMode::MonthYear => {
            let (year, month, _, _) = timestamp_parts(unix_secs);
            format!("{} {:02}", month_name(month), year % 100)
        }
    }
}

fn uses_time_only_axis(timeframe: Timeframe) -> bool {
    timeframe.duration_ms() <= Timeframe::H1.duration_ms()
}

fn format_time_of_day(unix_secs: u64) -> String {
    let secs_per_day: u64 = 86400;
    let secs_per_hour: u64 = 3600;
    let secs_per_minute: u64 = 60;

    let remaining = unix_secs % secs_per_day;
    let hours = remaining / secs_per_hour;
    let minutes = (remaining % secs_per_hour) / secs_per_minute;

    format!("{hours:02}:{minutes:02}")
}

fn month_name(month: usize) -> &'static str {
    const MONTH_NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    MONTH_NAMES
        .get(month.saturating_sub(1))
        .copied()
        .unwrap_or("Jan")
}

fn timestamp_parts(unix_secs: u64) -> (u64, usize, u64, u64) {
    let secs_per_day: u64 = 86400;
    let secs_per_hour: u64 = 3600;

    let total_days = unix_secs / secs_per_day;
    let remaining = unix_secs % secs_per_day;
    let hours = remaining / secs_per_hour;

    let mut year: u64 = 1970;
    let mut days_left = total_days;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }

    let month_days = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month: usize = 1;
    for (index, &days_in_month) in month_days.iter().enumerate() {
        if days_left < days_in_month {
            month = index + 1;
            break;
        }
        days_left -= days_in_month;
    }

    (year, month, days_left + 1, hours)
}

fn is_leap_year(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
