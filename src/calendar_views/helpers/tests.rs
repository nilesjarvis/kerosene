use super::*;
use chrono::{Duration, TimeZone};

fn event(title: &str, impact: &str, date: String) -> api::CalendarEvent {
    api::CalendarEvent {
        title: title.to_string(),
        country: "US".to_string(),
        date,
        impact: impact.to_string(),
        forecast: String::new(),
        previous: String::new(),
    }
}

fn filtered_titles(
    events: &[api::CalendarEvent],
    impact: CalendarImpactFilter,
    window: CalendarWindowFilter,
    now: DateTime<Local>,
) -> Vec<String> {
    filtered_events(
        &parsed_events(events),
        impact,
        window,
        now.with_timezone(&Utc),
        now,
    )
    .into_iter()
    .map(|(event, _)| event.title.clone())
    .collect()
}

fn next_title(events: &[api::CalendarEvent], now: DateTime<Utc>) -> Option<&str> {
    next_important_event(&parsed_events(events), now).map(|(event, _)| event.title.as_str())
}

#[test]
fn calendar_filters_preserve_unscheduled_events_cutoffs_and_stable_ties() {
    let now = Local
        .with_ymd_and_hms(2026, 6, 15, 12, 0, 0)
        .single()
        .expect("local noon");
    let date = |minutes| (now + Duration::minutes(minutes)).to_rfc3339();
    let events = vec![
        event("a", "High", date(-31)),
        event("b", "Low", date(120)),
        event("c", "High", date(60)),
        event("d", "Medium", date(15)),
        event("e", "Medium", date(-30)),
        event("f", "High", "zzz".to_string()),
        event("g", "Low", "aaa".to_string()),
        event("h", "High", date(60)),
        event("i", "Holiday", date(-2 * 24 * 60)),
    ];
    let cases: &[(CalendarImpactFilter, CalendarWindowFilter, &[&str])] = &[
        (
            CalendarImpactFilter::All,
            CalendarWindowFilter::Week,
            &["g", "f", "i", "a", "e", "d", "c", "h", "b"],
        ),
        (
            CalendarImpactFilter::MediumHigh,
            CalendarWindowFilter::Week,
            &["f", "a", "e", "d", "c", "h"],
        ),
        (
            CalendarImpactFilter::High,
            CalendarWindowFilter::Week,
            &["f", "a", "c", "h"],
        ),
        (
            CalendarImpactFilter::All,
            CalendarWindowFilter::Upcoming,
            &["g", "f", "e", "d", "c", "h", "b"],
        ),
        (
            CalendarImpactFilter::MediumHigh,
            CalendarWindowFilter::Upcoming,
            &["f", "e", "d", "c", "h"],
        ),
        (
            CalendarImpactFilter::High,
            CalendarWindowFilter::Upcoming,
            &["f", "c", "h"],
        ),
        (
            CalendarImpactFilter::All,
            CalendarWindowFilter::Today,
            &["g", "f", "a", "e", "d", "c", "h", "b"],
        ),
        (
            CalendarImpactFilter::MediumHigh,
            CalendarWindowFilter::Today,
            &["f", "a", "e", "d", "c", "h"],
        ),
        (
            CalendarImpactFilter::High,
            CalendarWindowFilter::Today,
            &["f", "a", "c", "h"],
        ),
    ];
    for &(impact, window, expected) in cases {
        assert_eq!(
            filtered_titles(&events, impact, window, now),
            expected,
            "{impact:?}, {window:?}"
        );
    }
    assert_eq!(next_title(&events, now.with_timezone(&Utc)), Some("d"));
}

#[test]
fn calendar_view_uses_full_timestamp_precision_and_first_next_event_tie() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 15, 12, 0, 0)
        .single()
        .expect("UTC noon");
    let events = vec![
        event("late", "High", "2026-06-15T12:00:00.900Z".to_string()),
        event(
            "early-first",
            "High",
            "2026-06-15T13:00:00.100+01:00".to_string(),
        ),
        event(
            "early-second",
            "High",
            "2026-06-15T13:00:00.100+01:00".to_string(),
        ),
    ];
    assert_eq!(
        filtered_titles(
            &events,
            CalendarImpactFilter::All,
            CalendarWindowFilter::Week,
            now.with_timezone(&Local)
        ),
        ["early-first", "early-second", "late"]
    );
    assert_eq!(next_title(&events, now), Some("early-first"));
    let at_now = [event("now", "Medium", now.to_rfc3339())];
    assert_eq!(next_title(&at_now, now), Some("now"));
    let irrelevant = [
        event("low", "Low", now.to_rfc3339()),
        event("past", "High", (now - Duration::seconds(1)).to_rfc3339()),
        event("invalid", "High", "unknown".to_string()),
    ];
    assert_eq!(next_title(&irrelevant, now), None);
    assert_eq!(next_title(&[], now), None);
    assert!(
        filtered_titles(
            &[],
            CalendarImpactFilter::All,
            CalendarWindowFilter::Week,
            now.with_timezone(&Local)
        )
        .is_empty()
    );
}
