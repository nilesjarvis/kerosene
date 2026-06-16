mod controls;
mod events;
mod helpers;

use crate::app_state::TradingTerminal;
use crate::app_time::{local_datetime_from_unix_ms, utc_datetime_from_unix_ms};
use crate::message::Message;
use iced::widget::{column, container, responsive, rule, scrollable};
use iced::{Element, Fill};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Economic calendar view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_calendar(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_calendar_sized(size.width)).into()
    }

    fn view_calendar_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let compact = available_width < 560.0;
        let medium = available_width < 860.0;
        let now_utc = utc_datetime_from_unix_ms(self.status_bar_now_ms);
        let now_local = local_datetime_from_unix_ms(self.status_bar_now_ms);

        let status_text = calendar_status_text(
            self.calendar_loading,
            !self.calendar_events.is_empty(),
            self.calendar_error.as_deref(),
            self.calendar_last_fetch,
            self.status_bar_now,
        );
        let status_color = if self.calendar_error.is_some() {
            theme.palette().danger
        } else {
            theme.extended_palette().background.weak.text
        };

        let filtered = helpers::filtered_events(
            &self.calendar_events,
            self.calendar_impact_filter,
            self.calendar_window_filter,
            now_utc,
            now_local,
        );
        let next_important = helpers::next_important_event(&self.calendar_events, now_utc);

        let mut content = column![
            self.view_calendar_top_bar(compact),
            self.view_calendar_filters(),
            self.view_calendar_status_row(status_text, status_color),
            self.view_calendar_summary(next_important, now_utc),
            rule::horizontal(1),
        ]
        .spacing(6);

        if !compact && !medium && !self.calendar_events.is_empty() {
            content = content.push(self.view_calendar_table_header());
        }

        let content = content.push(
            scrollable(self.view_calendar_event_list(compact, medium, filtered, now_utc))
                .id(iced::widget::Id::new("calendar_scroll")),
        );

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(8)
            .into()
    }
}

fn calendar_status_text(
    loading: bool,
    has_events: bool,
    error: Option<&str>,
    last_fetch: Option<Instant>,
    now: Instant,
) -> String {
    if loading && !has_events {
        "Loading events...".to_string()
    } else if loading {
        "Refreshing...".to_string()
    } else if let Some(err) = error {
        if has_events {
            format!("Showing last good data; refresh failed: {err}")
        } else {
            format!("Load failed: {err}")
        }
    } else if let Some(last_fetch) = last_fetch {
        let age = now.saturating_duration_since(last_fetch).as_secs();
        if age < 60 {
            "Updated just now".to_string()
        } else {
            format!("Updated {}m ago", age / 60)
        }
    } else {
        "Not loaded".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::calendar_status_text;
    use std::time::{Duration, Instant};

    #[test]
    fn calendar_status_uses_supplied_tick_clock_for_age() {
        let now = Instant::now();

        assert_eq!(
            calendar_status_text(false, true, None, Some(now - Duration::from_secs(30)), now),
            "Updated just now"
        );
        assert_eq!(
            calendar_status_text(false, true, None, Some(now - Duration::from_secs(125)), now),
            "Updated 2m ago"
        );
    }

    #[test]
    fn calendar_status_handles_loading_and_error_states() {
        let now = Instant::now();

        assert_eq!(
            calendar_status_text(true, false, None, None, now),
            "Loading events..."
        );
        assert_eq!(
            calendar_status_text(true, true, None, None, now),
            "Refreshing..."
        );
        assert_eq!(
            calendar_status_text(false, false, Some("network"), None, now),
            "Load failed: network"
        );
        assert_eq!(
            calendar_status_text(false, true, Some("network"), None, now),
            "Showing last good data; refresh failed: network"
        );
    }
}
