use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, format_size};
use crate::message::Message;
use crate::twap_state::{TwapChildOrder, TwapEvent, TwapOrder};
use iced::widget::{Column, column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Theme, window};

// ---------------------------------------------------------------------------
// TWAP Details Window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_twap_details(&self, window_id: window::Id) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(twap) = self
            .twap_orders
            .values()
            .find(|twap| twap.window_id == Some(window_id))
        else {
            return container(text("TWAP not found").size(13))
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into();
        };

        let content = column![
            twap_header(twap, &theme),
            rule::horizontal(1),
            twap_summary(twap, &theme),
            rule::horizontal(1),
            twap_child_orders(twap, &theme),
            rule::horizontal(1),
            twap_events(twap, &theme),
            rule::horizontal(1),
            twap_notes(&theme),
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

fn twap_header<'a>(twap: &TwapOrder, theme: &Theme) -> Element<'a, Message> {
    let side_color = if twap.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    row![
        text(format!("TWAP #{}", twap.id)).size(16).width(Fill),
        text(twap.side_label()).size(12).color(side_color),
        text(twap.display_coin.clone()).size(13),
        text(twap.status.label())
            .size(12)
            .color(theme.palette().primary),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn twap_summary<'a>(twap: &TwapOrder, theme: &Theme) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let duration_minutes = twap.duration.as_secs_f64() / 60.0;
    let avg_price = weighted_average_fill_price(twap)
        .map(format_price)
        .unwrap_or_else(|| "-".to_string());
    let fee: f64 = twap.child_orders.iter().map(|child| child.fee).sum();
    column![
        section_title("Summary", theme),
        row![
            metric("Filled", format_size(twap.filled_size), weak),
            metric("Target", format_size(twap.target_size), weak),
            metric(
                "Progress",
                format!("{:.1}%", twap.progress_fraction() * 100.0),
                weak
            ),
            metric("Average", avg_price, weak),
        ]
        .spacing(8),
        row![
            metric("Remaining", format_size(twap.remaining_size), weak),
            metric(
                "Slices",
                format!("{} / {}", twap.slices_sent, twap.slice_count),
                weak
            ),
            metric("Duration", format!("{duration_minutes:.1}m"), weak),
            metric("Fees", format!("{fee:.4}"), weak),
        ]
        .spacing(8),
        row![
            metric("Min", format_price(twap.min_price), weak),
            metric("Max", format_price(twap.max_price), weak),
            metric(
                "Randomize",
                if twap.randomize { "On" } else { "Off" }.to_string(),
                weak
            ),
            metric(
                "Reduce Only",
                if twap.reduce_only { "Yes" } else { "No" }.to_string(),
                weak
            ),
        ]
        .spacing(8),
    ]
    .spacing(6)
    .into()
}

fn twap_child_orders<'a>(twap: &TwapOrder, theme: &Theme) -> Element<'a, Message> {
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
        text(child.status.label()).size(11).color(weak).width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn twap_events<'a>(twap: &TwapOrder, theme: &Theme) -> Element<'a, Message> {
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

fn twap_notes<'a>(theme: &Theme) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    column![
        section_title("Operating Notes", theme),
        text("TWAP slices are bounded Limit IOC orders. They do not intentionally leave resting child orders behind.")
            .size(11)
            .color(weak),
        text("A slice is skipped when the current book cannot fill the full planned size inside the configured min/max range.")
            .size(11)
            .color(weak),
        text("Closing or switching charts does not affect the TWAP. Disconnecting or changing wallets stops future slices.")
            .size(11)
            .color(weak),
        text("Live TWAPs do not resume after app restart. Completed/stopped history is saved in Advanced Orders.")
            .size(11)
            .color(weak),
    ]
    .spacing(5)
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

fn weighted_average_fill_price(twap: &TwapOrder) -> Option<f64> {
    let mut size = 0.0;
    let mut notional = 0.0;
    for child in &twap.child_orders {
        let Some(price) = child.avg_price else {
            continue;
        };
        if child.filled_size > 0.0 && price.is_finite() && price > 0.0 {
            size += child.filled_size;
            notional += child.filled_size * price;
        }
    }
    (size > 0.0).then_some(notional / size)
}
