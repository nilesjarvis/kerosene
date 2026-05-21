use super::chrome::calendar_row_style;
use crate::api;
use crate::message::Message;
use iced::widget::{Space, column, container, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) struct CalendarRowLayout<'a> {
    pub(super) event: &'a api::CalendarEvent,
    pub(super) time_str: String,
    pub(super) rel_str: String,
    pub(super) details: String,
    pub(super) country: Element<'static, Message>,
    pub(super) impact: Element<'static, Message>,
    pub(super) event_text_color: Color,
    pub(super) secondary_text: Color,
    pub(super) row_bg: Color,
}

pub(super) fn view_compact_calendar_event_row<'a>(
    layout: CalendarRowLayout<'a>,
) -> Element<'a, Message> {
    container(
        column![
            row![
                text(layout.time_str)
                    .font(crate::app_fonts::monospace_font())
                    .size(12)
                    .color(layout.event_text_color),
                layout.country,
                layout.impact,
                Space::new().width(Fill),
                text(layout.rel_str).size(10).color(layout.secondary_text),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            text(&layout.event.title)
                .size(12)
                .color(layout.event_text_color),
            text(layout.details).size(10).color(layout.secondary_text),
        ]
        .spacing(3),
    )
    .padding([5, 6])
    .style(move |_theme: &Theme| calendar_row_style(layout.row_bg))
    .into()
}

pub(super) fn view_medium_calendar_event_row<'a>(
    layout: CalendarRowLayout<'a>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(layout.time_str)
                    .font(crate::app_fonts::monospace_font())
                    .size(12)
                    .color(layout.event_text_color),
                text(layout.rel_str).size(10).color(layout.secondary_text),
            ]
            .width(72),
            layout.country,
            layout.impact,
            column![
                text(&layout.event.title)
                    .size(12)
                    .color(layout.event_text_color),
                text(layout.details).size(10).color(layout.secondary_text),
            ]
            .width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 6])
    .style(move |_theme: &Theme| calendar_row_style(layout.row_bg))
    .into()
}

pub(super) fn view_full_calendar_event_row<'a>(
    layout: CalendarRowLayout<'a>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(layout.time_str)
                    .font(crate::app_fonts::monospace_font())
                    .size(12)
                    .color(layout.event_text_color),
                text(layout.rel_str).size(10).color(layout.secondary_text),
            ]
            .width(92),
            layout.country,
            layout.impact,
            text(&layout.event.title)
                .size(12)
                .color(layout.event_text_color)
                .width(Fill),
            text(if layout.event.forecast.is_empty() {
                "-"
            } else {
                &layout.event.forecast
            })
            .font(crate::app_fonts::monospace_font())
            .size(11)
            .color(layout.event_text_color)
            .width(72),
            text(if layout.event.previous.is_empty() {
                "-"
            } else {
                &layout.event.previous
            })
            .font(crate::app_fonts::monospace_font())
            .size(11)
            .color(layout.event_text_color)
            .width(72),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 6])
    .style(move |_theme: &Theme| calendar_row_style(layout.row_bg))
    .into()
}
