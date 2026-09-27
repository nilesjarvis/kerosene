use super::*;

fn event(title: &str, date: &str) -> CalendarEvent {
    CalendarEvent {
        title: title.to_string(),
        country: format!("country:{title}"),
        date: date.to_string(),
        impact: format!("impact:{title}"),
        forecast: format!("forecast:{title}"),
        previous: format!("previous:{title}"),
    }
}

#[test]
fn calendar_sort_preserves_second_precision_raw_date_ties_and_all_event_fields() {
    let mut events = vec![
        event("early-fraction", "2026-06-15T13:00:00.100+01:00"),
        event("earlier", "2026-06-15T11:59:59Z"),
        event("invalid-z", "zzz"),
        event("late-fraction-first", "2026-06-15T12:00:00.900Z"),
        event("exact-offset", "2026-06-15T14:00:00+02:00"),
        event("late-fraction-second", "2026-06-15T12:00:00.900Z"),
        event("invalid-a", "aaa"),
        event("later", "2026-06-15T12:00:01Z"),
        event("invalid-date", "2026-02-30T00:00:00Z"),
        event("empty", ""),
    ];
    let original = events.clone();
    sort_calendar_events(&mut events);
    assert_eq!(
        events
            .iter()
            .map(|event| event.title.as_str())
            .collect::<Vec<_>>(),
        [
            "empty",
            "invalid-date",
            "invalid-a",
            "invalid-z",
            "earlier",
            "late-fraction-first",
            "late-fraction-second",
            "early-fraction",
            "exact-offset",
            "later"
        ]
    );
    for sorted in &events {
        let original = original
            .iter()
            .find(|event| event.title == sorted.title)
            .expect("original event");
        assert_eq!(
            (
                &sorted.date,
                &sorted.country,
                &sorted.impact,
                &sorted.forecast,
                &sorted.previous
            ),
            (
                &original.date,
                &original.country,
                &original.impact,
                &original.forecast,
                &original.previous
            )
        );
    }
}

#[test]
fn calendar_sort_accepts_empty_and_single_event_lists() {
    sort_calendar_events(&mut []);
    let mut events = [event("single", "unknown")];
    sort_calendar_events(&mut events);
    assert_eq!(events[0].title, "single");
    assert_eq!(events[0].date, "unknown");
}
