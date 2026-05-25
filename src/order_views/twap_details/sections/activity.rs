use crate::helpers::{format_price, format_size};
use crate::message::Message;
use crate::twap_state::{TwapChildOrder, TwapEvent, TwapOrder};

use super::super::super::details::section_title;
use super::super::formatting::child_id_text;
use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// TWAP Activity
// ---------------------------------------------------------------------------

pub(in crate::order_views::twap_details) fn twap_child_orders<'a>(
    twap: &TwapOrder,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let mut rows = Column::new().spacing(4).push(
        row![
            text("#").size(10).color(weak).width(48),
            text("Size").size(10).color(weak).width(Fill),
            text("Limit").size(10).color(weak).width(Fill),
            text("Fill").size(10).color(weak).width(Fill),
            text("ID").size(10).color(weak).width(Fill),
            text("Status").size(10).color(weak).width(Fill),
        ]
        .spacing(8),
    );

    if twap.child_orders.is_empty() {
        rows = rows.push(
            text("No child slices have been sent yet")
                .size(11)
                .color(weak),
        );
    } else {
        for child in twap.child_orders.iter().rev().take(80) {
            rows = rows.push(child_row(child, twap.started_at, theme));
        }
    }

    column![section_title("Child Slices", theme), rows]
        .spacing(6)
        .into()
}

pub(in crate::order_views::twap_details) fn twap_events<'a>(
    twap: &TwapOrder,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let mut rows = Column::new().spacing(4);
    if twap.events.is_empty() {
        rows = rows.push(text("No events recorded").size(11).color(weak));
    } else {
        for event in twap.events.iter().rev().take(80) {
            rows = rows.push(event_row(event, twap.started_at, theme));
        }
    }
    column![section_title("Event Log", theme), rows]
        .spacing(6)
        .into()
}

fn child_row<'a>(
    child: &TwapChildOrder,
    started_at: std::time::Instant,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let fill = if child.filled_size > 0.0 {
        format_size(child.filled_size)
    } else {
        "-".to_string()
    };
    let elapsed = child
        .requested_at
        .saturating_duration_since(started_at)
        .as_secs();
    row![
        text(format!("{} +{}s", child.index, elapsed))
            .size(11)
            .width(48),
        text(format_size(child.planned_size)).size(11).width(Fill),
        text(format_price(child.limit_price)).size(11).width(Fill),
        text(fill).size(11).width(Fill),
        text(child_id_text(child)).size(10).color(weak).width(Fill),
        text(child.status.label()).size(11).color(weak).width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn event_row<'a>(
    event: &TwapEvent,
    started_at: std::time::Instant,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let color = if event.is_error {
        theme.palette().danger
    } else {
        weak
    };
    let elapsed = event.at.saturating_duration_since(started_at).as_secs();
    row![
        text(format!("+{elapsed}s")).size(10).color(weak).width(56),
        text(format!("{:?}", event.kind))
            .size(10)
            .color(weak)
            .width(90),
        text(event.message.clone())
            .size(11)
            .color(color)
            .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
