mod controls;
mod events;
mod helpers;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{column, container, responsive, rule, scrollable};
use iced::{Element, Fill};

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
        let now_utc = chrono::Utc::now();
        let now_local = chrono::Local::now();

        let status_text = if self.calendar_loading && self.calendar_events.is_empty() {
            "Loading events...".to_string()
        } else if self.calendar_loading {
            "Refreshing...".to_string()
        } else if let Some(err) = &self.calendar_error {
            if self.calendar_events.is_empty() {
                format!("Load failed: {err}")
            } else {
                format!("Showing last good data; refresh failed: {err}")
            }
        } else if let Some(last) = self.calendar_last_fetch {
            let age = last.elapsed().as_secs();
            if age < 60 {
                "Updated just now".to_string()
            } else {
                format!("Updated {}m ago", age / 60)
            }
        } else {
            "Not loaded".to_string()
        };
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
