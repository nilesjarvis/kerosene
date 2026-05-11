use crate::advanced_order_history::{AdvancedOrderHistoryChild, AdvancedOrderHistoryEntry};
use crate::app_state::TradingTerminal;
use crate::helpers::{format_duration, format_price, format_size, format_timestamp_exact};
use crate::message::Message;
use iced::widget::{Column, column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Theme, window};

// ---------------------------------------------------------------------------
// Advanced Order History Details
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_advanced_order_history_details(
        &self,
        window_id: window::Id,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(entry_id) = self.advanced_order_history_windows.get(&window_id) else {
            return missing_history_view();
        };
        let Some(entry) = self
            .advanced_order_history
            .iter()
            .find(|entry| entry.id == *entry_id)
        else {
            return missing_history_view();
        };

        let content = column![
            history_header(entry, &theme),
            rule::horizontal(1),
            history_summary(entry, &theme),
            rule::horizontal(1),
            history_children(entry, &theme),
            rule::horizontal(1),
            history_logs(entry, &theme),
        ]
        .spacing(10)
        .padding(12);

        container(
            scrollable(content).direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4),
            )),
        )
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            text_color: Some(theme.palette().text),
            ..Default::default()
        })
        .into()
    }
}

fn missing_history_view<'a>() -> Element<'a, Message> {
    container(text("Advanced order history not found").size(13))
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .into()
}

fn history_header<'a>(entry: &AdvancedOrderHistoryEntry, theme: &Theme) -> Element<'a, Message> {
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

fn history_summary<'a>(entry: &AdvancedOrderHistoryEntry, theme: &Theme) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let avg = entry
        .average_price
        .map(format_price)
        .unwrap_or_else(|| "-".to_string());
    let completed = if entry.completed_at_ms > 0 {
        format_timestamp_exact(entry.completed_at_ms)
    } else {
        "-".to_string()
    };
    let runtime = if entry.completed_at_ms > entry.started_at_ms {
        format_duration(entry.completed_at_ms - entry.started_at_ms)
    } else {
        "-".to_string()
    };

    let range = match (entry.min_price, entry.max_price) {
        (Some(min), Some(max)) => format!("{}-{}", format_price(min), format_price(max)),
        _ => "-".to_string(),
    };

    column![
        section_title("Summary", theme),
        row![
            metric("Filled", format_size(entry.filled_size), weak),
            metric("Target", format_size(entry.target_size), weak),
            metric("Average", avg, weak),
            metric("Status", entry.status.clone(), weak),
        ]
        .spacing(8),
        row![
            metric("Completed", completed, weak),
            metric("Runtime", runtime, weak),
            metric("Range", range, weak),
            metric(
                "Reduce Only",
                if entry.reduce_only { "Yes" } else { "No" }.to_string(),
                weak,
            ),
        ]
        .spacing(8),
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
    ]
    .spacing(6)
    .into()
}

fn history_children<'a>(entry: &AdvancedOrderHistoryEntry, theme: &Theme) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let mut rows = Column::new().spacing(4).push(
        row![
            text("#").size(10).color(weak).width(48),
            text("Size").size(10).color(weak).width(Fill),
            text("Limit").size(10).color(weak).width(Fill),
            text("Fill").size(10).color(weak).width(Fill),
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
        text(child.status.clone()).size(11).color(weak).width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn history_logs<'a>(entry: &AdvancedOrderHistoryEntry, theme: &Theme) -> Element<'a, Message> {
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

fn section_title<'a>(label: &'static str, theme: &Theme) -> iced::widget::Text<'a> {
    text(label)
        .size(12)
        .color(theme.extended_palette().background.weak.text)
}

fn metric<'a>(label: &'static str, value: String, weak: iced::Color) -> Element<'a, Message> {
    container(column![text(label).size(10).color(weak), text(value).size(12)].spacing(2))
        .padding([6, 8])
        .width(Fill)
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        })
        .into()
}
