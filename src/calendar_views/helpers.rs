use crate::api;
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use chrono::{DateTime, Local, Utc};
use iced::{Color, Theme};

#[cfg(test)]
mod tests;

pub(super) type DatedEvent<'a> = (&'a api::CalendarEvent, Option<DateTime<Utc>>);

pub(super) fn parsed_events(events: &[api::CalendarEvent]) -> Vec<DatedEvent<'_>> {
    events
        .iter()
        .map(|event| {
            let dt = DateTime::parse_from_rfc3339(&event.date)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            (event, dt)
        })
        .collect()
}

pub(super) fn impact_rank(impact: &str) -> u8 {
    let lower = impact.to_ascii_lowercase();
    if lower.contains("high") {
        3
    } else if lower.contains("medium") {
        2
    } else if lower.contains("low") {
        1
    } else {
        0
    }
}

pub(super) fn impact_color(impact: &str, theme: &Theme) -> Color {
    let lower = impact.to_ascii_lowercase();
    if lower.contains("high") {
        theme.palette().danger
    } else if lower.contains("medium") {
        theme.palette().warning
    } else if lower.contains("holiday") {
        theme.extended_palette().background.weak.text
    } else {
        theme.extended_palette().background.strong.text
    }
}

pub(super) fn relative_time(dt_utc: DateTime<Utc>, now_utc: DateTime<Utc>) -> String {
    let duration = dt_utc.signed_duration_since(now_utc);
    let is_past = duration.num_seconds() < 0;
    let seconds = duration.num_seconds().abs();
    let minutes = (seconds / 60) % 60;
    let hours = (seconds / 3600) % 24;
    let days = seconds / 86400;
    let rel = if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    };
    if is_past {
        format!("{rel} ago")
    } else {
        format!("in {rel}")
    }
}

pub(super) fn filtered_events<'a>(
    events: &[DatedEvent<'a>],
    impact_filter: CalendarImpactFilter,
    window_filter: CalendarWindowFilter,
    now_utc: DateTime<Utc>,
    now_local: DateTime<Local>,
) -> Vec<DatedEvent<'a>> {
    let mut filtered: Vec<_> = events
        .iter()
        .copied()
        .filter_map(|(event, dt)| {
            let rank = impact_rank(&event.impact);
            let impact_ok = match impact_filter {
                CalendarImpactFilter::All => true,
                CalendarImpactFilter::High => rank >= 3,
                CalendarImpactFilter::MediumHigh => rank >= 2,
            };
            if !impact_ok {
                return None;
            }

            let time_ok = match (window_filter, dt) {
                (CalendarWindowFilter::Week, _) => true,
                (CalendarWindowFilter::Upcoming, Some(dt)) => {
                    dt >= now_utc - chrono::Duration::minutes(30)
                }
                (CalendarWindowFilter::Today, Some(dt)) => {
                    dt.with_timezone(&Local).date_naive() == now_local.date_naive()
                }
                (_, None) => true,
            };

            time_ok.then_some((event, dt))
        })
        .collect();

    filtered.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.date.cmp(&b.0.date)));
    filtered
}

pub(super) fn next_important_event<'a>(
    events: &[DatedEvent<'a>],
    now_utc: DateTime<Utc>,
) -> Option<(&'a api::CalendarEvent, DateTime<Utc>)> {
    events
        .iter()
        .copied()
        .filter_map(|(event, dt)| {
            let dt = dt?;
            (dt >= now_utc && impact_rank(&event.impact) >= 2).then_some((event, dt))
        })
        .min_by_key(|(_, dt)| *dt)
}

// Softer rules keep the dense table readable in both light and dark themes.
pub(super) fn separator_style(theme: &Theme) -> iced::widget::rule::Style {
    iced::widget::rule::Style {
        color: theme
            .extended_palette()
            .background
            .strong
            .color
            .scale_alpha(0.4),
        ..iced::widget::rule::default(theme)
    }
}
