use super::helpers::relative_time;
use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use chrono::{DateTime, Utc};
use iced::widget::{Column, container, text};
use iced::{Element, Fill};

mod row;

impl TradingTerminal {
    pub(crate) fn view_calendar_summary(
        &self,
        next_important: Option<(&api::CalendarEvent, DateTime<Utc>)>,
        now_utc: DateTime<Utc>,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        if let Some((event, dt)) = next_important {
            let local_dt = dt.with_timezone(&chrono::Local);
            let line = format!(
                "Next: {} {} {} ({})",
                local_dt.format("%a %H:%M"),
                event.country,
                event.title,
                relative_time(dt, now_utc)
            );
            text(line)
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into()
        } else {
            text("No upcoming medium/high events in this feed")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into()
        }
    }

    pub(crate) fn view_calendar_event_list<'a>(
        &'a self,
        compact: bool,
        medium: bool,
        filtered: Vec<(&'a api::CalendarEvent, Option<DateTime<Utc>>)>,
        now_utc: DateTime<Utc>,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let mut list = Column::new().spacing(4);
        let mut current_day = String::new();

        if filtered.is_empty() && !self.calendar_loading {
            list = list.push(
                container(
                    text("No matching events")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .padding([12, 4])
                .center_x(Fill),
            );
        }

        for (event, dt) in filtered {
            let local_dt = dt.map(|dt| dt.with_timezone(&chrono::Local));
            let day_str = local_dt
                .map(|dt| dt.format("%A, %b %e").to_string())
                .unwrap_or_else(|| "Unscheduled".to_string());
            let time_str = local_dt
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_else(|| "--:--".to_string());
            let rel_str = dt.map(|dt| relative_time(dt, now_utc)).unwrap_or_default();
            let is_past = dt.is_some_and(|dt| dt < now_utc);

            if day_str != current_day {
                current_day = day_str.clone();
                list = list.push(
                    container(text(day_str).size(12).color(theme.palette().primary))
                        .padding([8, 0])
                        .width(Fill),
                );
            }

            list = list.push(
                self.view_calendar_event_row(compact, medium, event, time_str, rel_str, is_past),
            );
        }

        list
    }
}
