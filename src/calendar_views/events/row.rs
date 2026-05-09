mod chrome;
mod layouts;

use self::chrome::calendar_badge;
use self::layouts::{
    CalendarRowLayout, view_compact_calendar_event_row, view_full_calendar_event_row,
    view_medium_calendar_event_row,
};
use super::super::helpers::impact_color;
use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Color, Element};

impl TradingTerminal {
    pub(super) fn view_calendar_event_row<'a>(
        &'a self,
        compact: bool,
        medium: bool,
        event: &'a api::CalendarEvent,
        time_str: String,
        rel_str: String,
        is_past: bool,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let muted = if is_past { 0.55 } else { 1.0 };
        let event_text_color = Color {
            a: muted,
            ..theme.palette().text
        };
        let secondary_text = Color {
            a: muted * 0.75,
            ..theme.palette().text
        };
        let row_bg = if is_past {
            Color {
                a: 0.10,
                ..theme.extended_palette().background.strong.color
            }
        } else {
            Color::TRANSPARENT
        };
        let impact = calendar_badge(
            &event.impact,
            Color {
                a: if is_past { 0.55 } else { 0.90 },
                ..impact_color(&event.impact, &theme)
            },
            theme.palette().background,
            76.0,
        );
        let country = calendar_badge(
            &event.country,
            theme.extended_palette().background.strong.color,
            theme.palette().text,
            50.0,
        );

        let details = if event.forecast.is_empty() && event.previous.is_empty() {
            String::new()
        } else {
            format!(
                "F {}  P {}",
                if event.forecast.is_empty() {
                    "-"
                } else {
                    &event.forecast
                },
                if event.previous.is_empty() {
                    "-"
                } else {
                    &event.previous
                }
            )
        };

        let layout = CalendarRowLayout {
            event,
            time_str,
            rel_str,
            details,
            country,
            impact,
            event_text_color,
            secondary_text,
            row_bg,
        };

        if compact {
            view_compact_calendar_event_row(layout)
        } else if medium {
            view_medium_calendar_event_row(layout)
        } else {
            view_full_calendar_event_row(layout)
        }
    }
}
