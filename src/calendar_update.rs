use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use chrono::{DateTime, Local, Utc};
use iced::Task;
use std::time::{Duration, Instant};

const CALENDAR_REFRESH_INTERVAL_SECS: u64 = 15 * 60;

impl TradingTerminal {
    pub(crate) fn update_calendar(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshCalendar => {
                return self.request_calendar_refresh(true);
            }
            Message::CalendarLoaded(request_id, result) => {
                if !self.calendar_loading || request_id != self.calendar_request_id {
                    return Task::none();
                }

                let now = Instant::now();
                self.calendar_loading = false;
                match result.into_result() {
                    Ok(events) => {
                        self.calendar_events = events;
                        self.calendar_error = None;
                        self.calendar_last_fetch = Some(now);
                        self.calendar_retry_attempts = 0;
                        self.calendar_next_retry = None;

                        let now = Utc::now();
                        let mut offset_y: f32 = 0.0;
                        let mut current_day = String::new();
                        let row_height: f32 = 40.0;
                        let spacing: f32 = 8.0;
                        let header_height: f32 = 44.0;

                        for event in self.calendar_events.iter() {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(&event.date) {
                                let local_dt = dt.with_timezone(&Local);
                                let day_str = local_dt.format("%A, %b %e").to_string();

                                if day_str != current_day {
                                    current_day = day_str;
                                    offset_y += header_height;
                                }

                                if dt.with_timezone(&Utc) > now {
                                    break;
                                }

                                offset_y += row_height + spacing + 1.0;
                            }
                        }

                        let final_offset = (offset_y - 200.0).max(0.0);

                        return iced::widget::operation::scroll_to(
                            iced::widget::Id::new("calendar_scroll"),
                            iced::widget::scrollable::AbsoluteOffset {
                                x: None,
                                y: Some(final_offset),
                            },
                        )
                        .map(|_: ()| Message::NoOp);
                    }
                    Err(e) => {
                        self.calendar_error = Some(redact_sensitive_response_text(&e));
                        let delay_secs =
                            Self::calendar_retry_delay_secs(self.calendar_retry_attempts);
                        self.calendar_retry_attempts =
                            self.calendar_retry_attempts.saturating_add(1).min(6);
                        self.calendar_next_retry = Some(now + Duration::from_secs(delay_secs));
                    }
                }
            }
            Message::Tick
                if self.is_calendar_open()
                    && !self.calendar_loading
                    && calendar_refresh_is_due(self.calendar_last_fetch, self.status_bar_now) =>
            {
                return self.request_calendar_refresh(false);
            }
            _ => {}
        }

        Task::none()
    }
}

fn calendar_refresh_is_due(last_fetch: Option<Instant>, now: Instant) -> bool {
    last_fetch.is_none_or(|last_fetch| {
        now.saturating_duration_since(last_fetch).as_secs() >= CALENDAR_REFRESH_INTERVAL_SECS
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::CalendarEvent;
    use iced::widget::pane_grid;

    fn event(title: &str) -> CalendarEvent {
        CalendarEvent {
            title: title.to_string(),
            country: "US".to_string(),
            date: "2026-06-12T12:00:00+00:00".to_string(),
            impact: "High".to_string(),
            forecast: String::new(),
            previous: String::new(),
        }
    }

    fn terminal_with_calendar_pane() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        let (panes, _) = pane_grid::State::new(crate::pane_state::PaneKind::Calendar);
        terminal.panes = panes;
        terminal
    }

    #[test]
    fn calendar_refresh_allocates_request_id() {
        let mut terminal = terminal_with_calendar_pane();

        let _task = terminal.request_calendar_refresh(false);

        assert!(terminal.calendar_loading);
        assert_eq!(terminal.calendar_request_id, 1);
    }

    #[test]
    fn calendar_refresh_wraps_request_id_without_replacing_active_owner() {
        let mut terminal = terminal_with_calendar_pane();
        terminal.calendar_request_id = u64::MAX;

        let _task = terminal.request_calendar_refresh(false);

        assert!(terminal.calendar_loading);
        assert_eq!(terminal.calendar_request_id, 0);

        let _duplicate_task = terminal.request_calendar_refresh(true);

        assert!(terminal.calendar_loading);
        assert_eq!(terminal.calendar_request_id, 0);
    }

    #[test]
    fn calendar_refresh_owner_survives_pane_reconstruction() {
        let mut terminal = terminal_with_calendar_pane();
        let _task = terminal.request_calendar_refresh(false);
        let request_id = terminal.calendar_request_id;

        let (panes, _) = pane_grid::State::new(crate::pane_state::PaneKind::Chart(0));
        terminal.panes = panes;
        let _closed_task = terminal.request_calendar_refresh(false);

        let (panes, _) = pane_grid::State::new(crate::pane_state::PaneKind::Calendar);
        terminal.panes = panes;
        let _reopened_task = terminal.request_calendar_refresh(false);

        assert!(terminal.calendar_loading);
        assert_eq!(terminal.calendar_request_id, request_id);

        let _completion_task = terminal.update_calendar(Message::CalendarLoaded(
            request_id,
            Ok(vec![event("current")]).into(),
        ));

        assert!(!terminal.calendar_loading);
        assert_eq!(terminal.calendar_events[0].title, "current");
    }

    #[test]
    fn stale_loaded_message_does_not_clear_current_refresh() {
        let mut terminal = terminal_with_calendar_pane();
        terminal.calendar_loading = true;
        terminal.calendar_request_id = 2;
        terminal.calendar_error = Some("current error".to_string());

        let _task =
            terminal.update_calendar(Message::CalendarLoaded(1, Ok(vec![event("stale")]).into()));

        assert!(terminal.calendar_loading);
        assert!(terminal.calendar_events.is_empty());
        assert_eq!(terminal.calendar_error.as_deref(), Some("current error"));
        assert!(terminal.calendar_last_fetch.is_none());
    }

    #[test]
    fn stale_error_does_not_change_current_cache_or_retry_owner() {
        let mut terminal = terminal_with_calendar_pane();
        let last_fetch = Instant::now();
        let next_retry = last_fetch + Duration::from_secs(60);
        terminal.calendar_loading = true;
        terminal.calendar_request_id = 9;
        terminal.calendar_events = vec![event("current")];
        terminal.calendar_error = Some("current error".to_string());
        terminal.calendar_last_fetch = Some(last_fetch);
        terminal.calendar_retry_attempts = 4;
        terminal.calendar_next_retry = Some(next_retry);

        let _task = terminal.update_calendar(Message::CalendarLoaded(
            8,
            Err("stale error".to_string()).into(),
        ));

        assert!(terminal.calendar_loading);
        assert_eq!(terminal.calendar_events[0].title, "current");
        assert_eq!(terminal.calendar_error.as_deref(), Some("current error"));
        assert_eq!(terminal.calendar_last_fetch, Some(last_fetch));
        assert_eq!(terminal.calendar_retry_attempts, 4);
        assert_eq!(terminal.calendar_next_retry, Some(next_retry));
    }

    #[test]
    fn matching_loaded_message_preserves_exact_event_and_clears_retry_state() {
        let mut terminal = terminal_with_calendar_pane();
        terminal.calendar_loading = true;
        terminal.calendar_request_id = 11;
        terminal.calendar_error = Some("prior error".to_string());
        terminal.calendar_retry_attempts = 4;
        terminal.calendar_next_retry = Some(Instant::now() + Duration::from_secs(60));
        let expected = CalendarEvent {
            title: "Exact event".to_string(),
            country: "GB".to_string(),
            date: "2026-06-12T14:30:00+00:00".to_string(),
            impact: "Medium".to_string(),
            forecast: "1.25%".to_string(),
            previous: "1.20%".to_string(),
        };

        let _task = terminal.update_calendar(Message::CalendarLoaded(
            11,
            Ok(vec![expected.clone()]).into(),
        ));

        assert!(!terminal.calendar_loading);
        assert_eq!(terminal.calendar_events.len(), 1);
        let actual = &terminal.calendar_events[0];
        assert_eq!(actual.title, expected.title);
        assert_eq!(actual.country, expected.country);
        assert_eq!(actual.date, expected.date);
        assert_eq!(actual.impact, expected.impact);
        assert_eq!(actual.forecast, expected.forecast);
        assert_eq!(actual.previous, expected.previous);
        assert!(terminal.calendar_error.is_none());
        assert!(terminal.calendar_last_fetch.is_some());
        assert_eq!(terminal.calendar_retry_attempts, 0);
        assert!(terminal.calendar_next_retry.is_none());
    }

    #[test]
    fn duplicate_loaded_message_after_completion_is_ignored() {
        let mut terminal = terminal_with_calendar_pane();
        terminal.calendar_loading = false;
        terminal.calendar_request_id = 7;
        terminal.calendar_events = vec![event("accepted")];

        let _task = terminal.update_calendar(Message::CalendarLoaded(
            7,
            Ok(vec![event("duplicate")]).into(),
        ));

        assert_eq!(terminal.calendar_events.len(), 1);
        assert_eq!(terminal.calendar_events[0].title, "accepted");
    }

    #[test]
    fn calendar_loaded_error_redacts_error_and_schedules_retry() {
        let mut terminal = terminal_with_calendar_pane();
        terminal.calendar_loading = true;
        terminal.calendar_request_id = 3;

        let _task = terminal.update_calendar(Message::CalendarLoaded(
            3,
            Err("calendar failed: api_key=calendar-secret".to_string()).into(),
        ));

        assert!(!terminal.calendar_loading);
        let error = terminal.calendar_error.as_deref().expect("calendar error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(!error.contains("calendar-secret"));
        assert_eq!(terminal.calendar_retry_attempts, 1);
        assert!(terminal.calendar_next_retry.is_some());
    }

    #[test]
    fn calendar_refresh_due_uses_supplied_tick_clock() {
        let now = Instant::now();

        assert!(calendar_refresh_is_due(None, now));
        assert!(!calendar_refresh_is_due(
            Some(now - Duration::from_secs(CALENDAR_REFRESH_INTERVAL_SECS - 1)),
            now
        ));
        assert!(calendar_refresh_is_due(
            Some(now - Duration::from_secs(CALENDAR_REFRESH_INTERVAL_SECS)),
            now
        ));
        assert!(!calendar_refresh_is_due(
            Some(now + Duration::from_secs(1)),
            now
        ));
    }
}
