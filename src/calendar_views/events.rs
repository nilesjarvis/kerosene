use super::helpers::{relative_time, separator_style};
use super::{DATE_WIDTH, ROW_HEIGHT};
use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use chrono::{DateTime, Utc};
use iced::widget::{Column, Space, column, container, rule, text, tooltip};
use iced::{Element, Fill};

mod row;

impl TradingTerminal {
    pub(crate) fn view_calendar_summary(
        &self,
        next_important: Option<(&api::CalendarEvent, DateTime<Utc>)>,
        now_utc: DateTime<Utc>,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let line = if let Some((event, dt)) = next_important {
            let local_dt = dt.with_timezone(&self.display_timezone.0);
            format!(
                "Next: {} {} {} · {}",
                local_dt.format("%a %H:%M"),
                event.country,
                event.title,
                relative_time(dt, now_utc)
            )
        } else {
            "No upcoming medium/high events in this feed".to_string()
        };

        tooltip(
            container(
                text(line.clone())
                    .size(11)
                    .wrapping(text::Wrapping::None)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(15)
            .clip(true),
            container(text(line).size(11)).padding(6).max_width(420),
            tooltip::Position::Top,
        )
        .style(container::rounded_box)
        .into()
    }

    pub(crate) fn view_calendar_event_list<'a>(
        &'a self,
        compact: bool,
        filtered: Vec<(&'a api::CalendarEvent, Option<DateTime<Utc>>)>,
        now_utc: DateTime<Utc>,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let mut list = Column::new().width(Fill);
        let mut current_day = None;
        let today = now_utc.with_timezone(&self.display_timezone.0).date_naive();

        if filtered.is_empty() {
            let label = if self.calendar_loading {
                "Loading events…"
            } else if self.calendar_events.is_empty() && self.calendar_error.is_some() {
                "Events unavailable"
            } else {
                "No matching events"
            };
            return list.push(
                container(
                    text(label)
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .padding([20, 8])
                .center_x(Fill),
            );
        }

        for (event, dt) in filtered {
            let local_dt = dt.map(|dt| dt.with_timezone(&self.display_timezone.0));
            let day = local_dt.map(|dt| dt.date_naive());
            let starts_day = current_day != Some(day);
            current_day = Some(day);
            let time_str = local_dt
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_else(|| "TBD".to_string());
            let rel_str = dt
                .map(|dt| relative_time(dt, now_utc))
                .unwrap_or_else(|| "Time not available".to_string());
            let is_past = dt.is_some_and(|dt| dt < now_utc);
            let date_color = if day == Some(today) {
                theme.palette().primary
            } else {
                theme.palette().text
            };
            let event_row =
                self.view_calendar_event_row(compact, event, time_str, rel_str, is_past);

            if compact {
                if starts_day {
                    let date = local_dt
                        .map(|dt| dt.format("%a, %b %-d").to_string().to_uppercase())
                        .unwrap_or_else(|| "UNSCHEDULED".to_string());
                    list = list.push(
                        container(
                            text(date)
                                .size(10)
                                .font(crate::app_fonts::monospace_font())
                                .color(date_color),
                        )
                        .padding([5, 8])
                        .width(Fill)
                        .style(|theme: &iced::Theme| container::Style {
                            background: Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .weak
                                    .color
                                    .scale_alpha(0.5)
                                    .into(),
                            ),
                            ..Default::default()
                        }),
                    );
                }
                list = list
                    .push(event_row)
                    .push(rule::horizontal(1).style(separator_style));
            } else {
                let date: Element<'_, Message> = if starts_day {
                    list = list.push(rule::horizontal(1).style(separator_style));
                    let weekday = local_dt
                        .map(|dt| dt.format("%a").to_string().to_uppercase())
                        .unwrap_or_else(|| "TBD".to_string());
                    let month_day = local_dt
                        .map(|dt| dt.format("%b %-d").to_string().to_uppercase())
                        .unwrap_or_else(|| "—".to_string());
                    container(column![
                        text(weekday)
                            .size(10)
                            .line_height(1.2)
                            .font(crate::app_fonts::monospace_font())
                            .color(date_color),
                        text(month_day)
                            .size(10)
                            .line_height(1.2)
                            .font(crate::app_fonts::monospace_font())
                            .color(theme.extended_palette().background.weak.text),
                    ])
                    .padding([4, 8])
                    .width(DATE_WIDTH)
                    .height(ROW_HEIGHT)
                    .into()
                } else {
                    Space::new().width(DATE_WIDTH).height(ROW_HEIGHT).into()
                };
                list = list.push(
                    iced::widget::row![
                        date,
                        rule::vertical(1).style(separator_style),
                        column![event_row, rule::horizontal(1).style(separator_style)].width(Fill),
                    ]
                    .height(ROW_HEIGHT + 1.0),
                );
            }
        }

        list
    }
}
