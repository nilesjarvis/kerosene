use crate::helpers::not_available_placeholder;

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;

// ---------------------------------------------------------------------------
// Market Sessions
// ---------------------------------------------------------------------------

fn format_market_time_until(now_utc: DateTime<Utc>, target_utc: DateTime<Utc>) -> String {
    let diff = target_utc - now_utc;
    let total_minutes = diff.num_minutes().max(0);
    let days = total_minutes / (24 * 60);
    let hours = (total_minutes % (24 * 60)) / 60;
    let minutes = total_minutes % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else {
        format!("{hours}h {minutes}m")
    }
}

fn next_market_open(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    for day_offset in 0..8 {
        let date = local_now.date_naive() + Duration::days(day_offset);
        if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            continue;
        }
        let naive = date.and_hms_opt(hour, minute, 0)?;
        let open_local = tz
            .from_local_datetime(&naive)
            .earliest()
            .or_else(|| tz.from_local_datetime(&naive).latest())?;
        if open_local > local_now {
            return Some(open_local);
        }
    }
    None
}

fn current_market_close(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    let close_local = local_time_for_date(tz, local_now.date_naive(), hour, minute)?;
    (close_local > local_now).then_some(close_local)
}

pub(super) fn market_is_active(
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> bool {
    let local_now = now_utc.with_timezone(&tz);
    let date = local_now.date_naive();
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }

    let Some(open_local) = local_time_for_date(tz, date, open.0, open.1) else {
        return false;
    };
    let Some(close_local) = local_time_for_date(tz, date, close.0, close.1) else {
        return false;
    };

    local_now >= open_local && local_now < close_local
}

fn local_time_for_date(
    tz: Tz,
    date: chrono::NaiveDate,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    tz.from_local_datetime(&naive)
        .earliest()
        .or_else(|| tz.from_local_datetime(&naive).latest())
}

pub(super) fn market_clock_text(
    label: &str,
    now_utc: DateTime<Utc>,
    tz: Tz,
    open: (u32, u32),
    close: (u32, u32),
) -> String {
    let now_local = now_utc.with_timezone(&tz);
    let session_status = if market_is_active(now_utc, tz, open, close) {
        let close_in = current_market_close(now_utc, tz, close.0, close.1)
            .map(|dt| format_market_time_until(now_utc, dt.with_timezone(&Utc)))
            .unwrap_or_else(not_available_placeholder);
        format!("closes in {close_in}")
    } else {
        let next_open = next_market_open(now_utc, tz, open.0, open.1)
            .map(|dt| format_market_time_until(now_utc, dt.with_timezone(&Utc)))
            .unwrap_or_else(not_available_placeholder);
        format!("opens in {next_open}")
    };
    format!(
        "{label} {:02}:{:02}:{:02} {} ({session_status})",
        now_local.hour(),
        now_local.minute(),
        now_local.second(),
        now_local.format("%Z")
    )
}
