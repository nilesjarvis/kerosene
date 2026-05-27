use crate::advanced_order_history::{
    AdvancedOrderHistoryChild, AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind,
};
use crate::helpers::{format_duration, format_price, format_size, format_usd};
use crate::message::Message;

use super::super::details::{metric, section_title};
use super::formatting::{
    history_child_id, history_completed_text, history_price_range_text, history_runtime_text,
};

use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

pub(super) fn history_header<'a>(
    entry: &AdvancedOrderHistoryEntry,
    theme: &Theme,
) -> Element<'a, Message> {
    let side_color = if entry.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    row![
        text(format!("{} #{}", entry.kind.label(), entry.source_id))
            .size(16)
            .width(Fill),
        text(entry.side_label()).size(12).color(side_color),
        text(entry.display_coin.clone()).size(13),
        text(entry.status.clone())
            .size(12)
            .color(theme.palette().primary),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn history_summary<'a>(
    entry: &AdvancedOrderHistoryEntry,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let avg = entry
        .average_price
        .map(format_price)
        .unwrap_or_else(|| "-".to_string());
    let last_working = entry
        .last_working_price
        .map(format_price)
        .unwrap_or_else(|| "-".to_string());

    let base = column![
        section_title("Summary", theme),
        row![
            metric("Filled", format_size(entry.filled_size), weak),
            metric("Target", format_size(entry.target_size), weak),
            metric("Average", avg, weak),
            metric("Status", entry.status.clone(), weak),
        ]
        .spacing(8),
        row![
            metric("Completed", history_completed_text(entry), weak),
            metric("Runtime", history_runtime_text(entry), weak),
            metric(
                if entry.kind == AdvancedOrderHistoryKind::Chase {
                    "Last Working"
                } else {
                    "Range"
                },
                if entry.kind == AdvancedOrderHistoryKind::Chase {
                    last_working
                } else {
                    history_price_range_text(entry)
                },
                weak,
            ),
            metric(
                "Reduce Only",
                if entry.reduce_only { "Yes" } else { "No" }.to_string(),
                weak,
            ),
        ]
        .spacing(8)
    ]
    .spacing(6);

    if entry.kind == AdvancedOrderHistoryKind::Chase {
        return base
            .push(
                row![
                    metric("Reprices", entry.reprice_count.to_string(), weak),
                    metric("Notional", format_history_usd(entry.gross_notional), weak),
                    metric("Fees", format_history_usd(entry.total_fee), weak),
                    metric("Closed PnL", format_history_usd(entry.closed_pnl), weak),
                ]
                .spacing(8),
            )
            .into();
    }

    base.push(
        row![
            metric("Slices", entry.slices_sent.to_string(), weak),
            metric("Reprices", entry.reprice_count.to_string(), weak),
            metric(
                "Randomize",
                if entry.randomize { "On" } else { "Off" }.to_string(),
                weak,
            ),
            metric("Last", entry.summary.clone(), weak),
        ]
        .spacing(8),
    )
    .into()
}

fn format_history_usd(value: f64) -> String {
    if value.is_finite() {
        format_usd(&format!("{value:.2}"))
    } else {
        "-".to_string()
    }
}

pub(super) fn history_children<'a>(
    entry: &AdvancedOrderHistoryEntry,
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

    if entry.children.is_empty() {
        rows = rows.push(text("No child-order records").size(11).color(weak));
    } else {
        for child in entry.children.iter().rev().take(80) {
            rows = rows.push(history_child_row(child, theme));
        }
    }

    column![section_title("Child Orders", theme), rows]
        .spacing(6)
        .into()
}

fn history_child_row<'a>(child: &AdvancedOrderHistoryChild, theme: &Theme) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let fill = if child.filled_size > 0.0 {
        format_size(child.filled_size)
    } else {
        "-".to_string()
    };
    row![
        text(format!(
            "{} +{}",
            child.index,
            format_duration(child.elapsed_ms)
        ))
        .size(11)
        .width(48),
        text(format_size(child.planned_size)).size(11).width(Fill),
        text(format_price(child.limit_price)).size(11).width(Fill),
        text(fill).size(11).width(Fill),
        text(history_child_id(child))
            .size(10)
            .color(weak)
            .width(Fill),
        text(child.status.clone()).size(11).color(weak).width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn history_logs<'a>(
    entry: &AdvancedOrderHistoryEntry,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let mut rows = Column::new().spacing(4);
    if entry.logs.is_empty() {
        rows = rows.push(text("No log entries").size(11).color(weak));
    } else {
        for log in entry.logs.iter().rev().take(100) {
            let color = if log.is_error {
                theme.palette().danger
            } else {
                weak
            };
            rows = rows.push(
                row![
                    text(format!("+{}", format_duration(log.elapsed_ms)))
                        .size(10)
                        .color(weak)
                        .width(64),
                    text(log.kind.clone()).size(10).color(weak).width(90),
                    text(log.message.clone()).size(11).color(color).width(Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    column![section_title("Event Log", theme), rows]
        .spacing(6)
        .into()
}
