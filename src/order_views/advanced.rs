use crate::advanced_order_history::AdvancedOrderHistoryEntry;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, format_relative_time, format_size};
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseOrder};
use crate::twap_state::{TwapOrder, TwapStatus};
use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, row, rule, scrollable, text};
use iced::{Alignment, Color, Element, Fill, Point, Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Advanced Orders
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_advanced_orders(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let mut header = row![text("Advanced Orders").size(12).width(Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        if self.active_advanced_order_count() > 0 {
            header = header.push(stop_all_button());
        }

        let mut rows = Column::new().spacing(4);
        let active_twaps: Vec<_> = self
            .twap_orders
            .values()
            .filter(|twap| !twap.status.is_terminal())
            .collect();
        if self.chase_orders.is_empty() && active_twaps.is_empty() {
            rows = rows.push(
                container(
                    text("No active advanced orders")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            );
        } else {
            for chase in self.chase_orders.values() {
                rows = rows.push(chase_order_row(chase, &theme, self.spinner_phase));
            }
            for twap in active_twaps {
                rows = rows.push(twap_order_row(twap, &theme, self.spinner_phase));
            }
        }
        rows = rows.push(rule::horizontal(1));
        rows = rows.push(
            text("History")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        );
        if self.advanced_order_history.is_empty() {
            rows = rows.push(
                text("Completed advanced orders will appear here")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        } else {
            let now_ms = Self::now_ms();
            for entry in self.advanced_order_history.iter().take(40) {
                rows = rows.push(history_order_row(entry, &theme, now_ms));
            }
        }

        let content = column![
            header,
            rule::horizontal(1),
            scrollable(rows)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4)
                ))
                .height(Fill),
        ]
        .spacing(8);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(8)
            .into()
    }
}

fn chase_order_row(
    chase: &ChaseOrder,
    theme: &Theme,
    spinner_phase: f32,
) -> Element<'static, Message> {
    let side = if chase.is_buy { "BUY" } else { "SELL" };
    let side_color = if chase.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let weak_text = theme.extended_palette().background.weak.text;
    let status = chase_status(chase);
    let status_color = if chase.lifecycle.is_stopping() {
        theme.palette().danger
    } else if !matches!(chase.lifecycle, ChaseLifecycle::Resting)
    {
        theme.palette().primary
    } else {
        weak_text
    };
    let price = if chase.current_price.is_finite() && chase.current_price > 0.0 {
        format_price(chase.current_price)
    } else {
        "Loading".to_string()
    };
    let reduce_only = if chase.reduce_only { " | RO" } else { "" };
    let meta = format!("{} reprices{reduce_only}", chase.reprice_count);
    let size = if chase.target_size.is_finite() && chase.target_size > 0.0 {
        format!(
            "{}/{} rem {}",
            format_size(chase.filled_size),
            format_size(chase.target_size),
            format_size(chase.remaining_size)
        )
    } else {
        format_size(chase.remaining_size)
    };

    container(
        row![
            spinning_gear(spinner_phase, 13, theme.palette().primary),
            badge("CHASE", theme),
            text(side).size(10).color(side_color),
            text(chase.coin.clone()).size(12).width(Fill),
            text(format!("{size} @ {price}")).size(11),
            text(meta).size(10).color(weak_text),
            text(status).size(10).color(status_color),
            stop_button(chase.id)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(|theme: &Theme| container_style::Style {
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

fn twap_order_row(
    twap: &TwapOrder,
    theme: &Theme,
    spinner_phase: f32,
) -> Element<'static, Message> {
    let side = twap.side_label();
    let side_color = if twap.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let weak_text = theme.extended_palette().background.weak.text;
    let status_color = match twap.status {
        TwapStatus::Error | TwapStatus::CompletedPartial => theme.palette().danger,
        TwapStatus::Running
        | TwapStatus::WaitingForMarket
        | TwapStatus::Paused
        | TwapStatus::Stopping => theme.palette().primary,
        TwapStatus::Stopped | TwapStatus::Completed => weak_text,
    };
    let progress = format!(
        "{} / {}",
        format_size(twap.filled_size),
        format_size(twap.target_size)
    );
    let range = format!(
        "{}-{}",
        format_price(twap.min_price),
        format_price(twap.max_price)
    );
    let meta = format!(
        "{} of {} slices | {range}",
        twap.slices_sent, twap.slice_count
    );
    let status = twap_status_text(twap);
    let stop_cell = if twap.status.is_terminal() {
        details_button(twap.id)
    } else {
        row![details_button(twap.id), stop_twap_button(twap.id)]
            .spacing(4)
            .into()
    };

    container(
        row![
            spinning_gear(spinner_phase, 13, theme.palette().primary),
            badge("TWAP", theme),
            text(side).size(10).color(side_color),
            text(twap.coin.clone()).size(12).width(Fill),
            text(progress).size(11),
            text(meta).size(10).color(weak_text),
            text(status).size(10).color(status_color),
            stop_cell
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(|theme: &Theme| container_style::Style {
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

fn twap_status_text(twap: &TwapOrder) -> String {
    if let Some(reason) = twap.pause_reason {
        return format!("Paused: {}", reason.label());
    }
    twap.status.label().to_string()
}

fn history_order_row(
    entry: &AdvancedOrderHistoryEntry,
    theme: &Theme,
    now_ms: u64,
) -> Element<'static, Message> {
    let side_color = if entry.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let weak_text = theme.extended_palette().background.weak.text;
    let status_color = if entry.status == "Error" || entry.status == "Partial" {
        theme.palette().danger
    } else {
        weak_text
    };
    let progress = if entry.target_size > 0.0 {
        format!(
            "{} / {}",
            format_size(entry.filled_size),
            format_size(entry.target_size)
        )
    } else {
        format_size(entry.filled_size)
    };
    let completed = if entry.completed_at_ms > 0 {
        format!(
            "{} ago",
            format_relative_time(entry.completed_at_ms, now_ms)
        )
    } else {
        "saved".to_string()
    };
    let display_summary = hide_order_oid_references(&entry.summary);
    let summary = compact_summary(&display_summary);

    container(
        row![
            badge(entry.kind.label(), theme),
            text(entry.side_label()).size(10).color(side_color),
            text(entry.display_coin.clone()).size(12).width(70),
            text(summary).size(10).color(weak_text).width(Fill),
            text(progress).size(11),
            text(completed).size(10).color(weak_text),
            text(entry.status.clone()).size(10).color(status_color),
            button(
                text("Info")
                    .size(10)
                    .center()
                    .width(iced::Length::Fixed(36.0)),
            )
            .on_press(Message::OpenAdvancedOrderHistory(entry.id.clone()))
            .padding([3, 6])
            .style(history_info_button_style),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(|theme: &Theme| container_style::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.weak.color,
        },
        ..Default::default()
    })
    .into()
}

fn chase_status(chase: &ChaseOrder) -> &'static str {
    chase.lifecycle.label()
}

fn compact_summary(summary: &str) -> String {
    const LIMIT: usize = 140;
    if summary.chars().count() <= LIMIT {
        return summary.to_string();
    }
    summary.chars().take(LIMIT).collect::<String>() + "..."
}

fn hide_order_oid_references(summary: &str) -> String {
    strip_inline_oid_references(&strip_parenthesized_oid_references(summary))
}

fn strip_parenthesized_oid_references(summary: &str) -> String {
    let mut result = String::with_capacity(summary.len());
    let mut rest = summary;
    while let Some(start) = rest.find("(oid ") {
        result.push_str(rest[..start].trim_end());
        let after_prefix = &rest[start + "(oid ".len()..];
        let Some(end) = after_prefix.find(')') else {
            result.push_str(&rest[start..]);
            return result;
        };
        let order_id = &after_prefix[..end];
        if order_id.is_empty() || !order_id.chars().all(|ch| ch.is_ascii_digit()) {
            result.push_str(&rest[start..start + "(oid ".len()]);
            rest = after_prefix;
            continue;
        }
        rest = &after_prefix[end + 1..];
    }
    result.push_str(rest);
    result
}

fn strip_inline_oid_references(summary: &str) -> String {
    let mut words = summary.split_whitespace().peekable();
    let mut stripped = Vec::new();

    while let Some(word) = words.next() {
        if word.eq_ignore_ascii_case("oid")
            && let Some(next) = words.peek()
            && let Some(suffix) = numeric_token_suffix(next)
        {
            words.next();
            stripped.push(format!("order{suffix}"));
            continue;
        }

        if let Some(suffix) = oid_assignment_suffix(word) {
            stripped.push(format!("order{suffix}"));
            continue;
        }

        stripped.push(word.to_string());
    }

    stripped.join(" ")
}

fn oid_assignment_suffix(word: &str) -> Option<&str> {
    let (label, value) = word.split_once('=')?;
    if label.eq_ignore_ascii_case("oid") {
        numeric_token_suffix(value)
    } else {
        None
    }
}

fn numeric_token_suffix(value: &str) -> Option<&str> {
    let digit_end = value
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_ascii_digit()).then_some(index))
        .unwrap_or(value.len());
    (digit_end > 0).then_some(&value[digit_end..])
}

#[cfg(test)]
mod tests {
    use super::hide_order_oid_references;

    #[test]
    fn hides_parenthesized_oid_from_history_summary() {
        assert_eq!(
            hide_order_oid_references("Chase filled: BUY 0.3 BTC @ $106 (oid 42)"),
            "Chase filled: BUY 0.3 BTC @ $106"
        );
        assert_eq!(
            hide_order_oid_references("Resting (oid 42); Error: rejected"),
            "Resting; Error: rejected"
        );
    }

    #[test]
    fn hides_inline_oid_from_history_summary() {
        assert_eq!(
            hide_order_oid_references("Slice 2 unexpectedly rested as oid 123; cancelling"),
            "Slice 2 unexpectedly rested as order; cancelling"
        );
        assert_eq!(
            hide_order_oid_references("filled oid=123 cloid=0xabc"),
            "filled order cloid=0xabc"
        );
    }
}

fn stop_button(chase_id: u64) -> Element<'static, Message> {
    button(
        text("Stop")
            .size(10)
            .center()
            .width(iced::Length::Fixed(44.0)),
    )
    .on_press(Message::StopChaseById(chase_id))
    .padding([3, 6])
    .style(|theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme.extended_palette().background.strong.color,
            _ => theme.extended_palette().background.weak.color,
        };
        let danger = theme.palette().danger;
        button::Style {
            background: Some(bg.into()),
            text_color: danger,
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.45, ..danger },
            },
            ..Default::default()
        }
    })
    .into()
}

fn stop_twap_button(twap_id: u64) -> Element<'static, Message> {
    stop_like_button("Stop", Message::StopTwap(twap_id))
}

fn details_button(twap_id: u64) -> Element<'static, Message> {
    button(
        text("Info")
            .size(10)
            .center()
            .width(iced::Length::Fixed(36.0)),
    )
    .on_press(Message::OpenTwapDetails(twap_id))
    .padding([3, 6])
    .style(|theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme.extended_palette().background.strong.color,
            _ => theme.extended_palette().background.weak.color,
        };
        let primary = theme.palette().primary;
        button::Style {
            background: Some(bg.into()),
            text_color: primary,
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.45, ..primary },
            },
            ..Default::default()
        }
    })
    .into()
}

fn history_info_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    };
    let primary = theme.palette().primary;
    button::Style {
        background: Some(bg.into()),
        text_color: primary,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color { a: 0.45, ..primary },
        },
        ..Default::default()
    }
}

fn stop_all_button() -> Element<'static, Message> {
    button(text("Stop All").size(10).center())
        .on_press(Message::StopAllAdvancedOrders)
        .padding([3, 8])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            let danger = theme.palette().danger;
            button::Style {
                background: Some(bg.into()),
                text_color: danger,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color { a: 0.45, ..danger },
                },
                ..Default::default()
            }
        })
        .into()
}

fn stop_like_button(label: &'static str, message: Message) -> Element<'static, Message> {
    button(
        text(label)
            .size(10)
            .center()
            .width(iced::Length::Fixed(44.0)),
    )
    .on_press(message)
    .padding([3, 6])
    .style(|theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme.extended_palette().background.strong.color,
            _ => theme.extended_palette().background.weak.color,
        };
        let danger = theme.palette().danger;
        button::Style {
            background: Some(bg.into()),
            text_color: danger,
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.45, ..danger },
            },
            ..Default::default()
        }
    })
    .into()
}

fn badge(label: &'static str, _theme: &Theme) -> Element<'static, Message> {
    container(text(label).size(9).center())
        .padding([2, 3])
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            text_color: Some(theme.extended_palette().background.strong.text),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn spinning_gear(phase: f32, size: u32, color: Color) -> Element<'static, Message> {
    container(
        iced::widget::canvas(SpinningGear { phase, color })
            .width(size as f32)
            .height(size as f32),
    )
    .width(size)
    .height(size)
    .center(Fill)
    .into()
}

struct SpinningGear {
    phase: f32,
    color: Color,
}

impl canvas::Program<Message> for SpinningGear {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 1.2;
        if radius <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let stroke = canvas::Stroke::default()
            .with_color(self.color)
            .with_width(1.3)
            .with_line_cap(canvas::LineCap::Round);
        let muted_stroke = canvas::Stroke::default()
            .with_color(Color {
                a: 0.45,
                ..self.color
            })
            .with_width(1.0)
            .with_line_cap(canvas::LineCap::Round);

        frame.stroke(&canvas::Path::circle(center, radius * 0.58), stroke);
        frame.stroke(&canvas::Path::circle(center, radius * 0.22), muted_stroke);

        for i in 0..8 {
            let angle = self.phase + i as f32 * std::f32::consts::TAU / 8.0;
            let inner = radial_point(center, radius * 0.72, angle);
            let outer = radial_point(center, radius, angle);
            let tooth = canvas::Path::new(|path| {
                path.move_to(inner);
                path.line_to(outer);
            });
            frame.stroke(&tooth, stroke);
        }

        for i in 0..4 {
            let angle = self.phase + i as f32 * std::f32::consts::TAU / 4.0;
            let inner = radial_point(center, radius * 0.28, angle);
            let outer = radial_point(center, radius * 0.52, angle);
            let spoke = canvas::Path::new(|path| {
                path.move_to(inner);
                path.line_to(outer);
            });
            frame.stroke(&spoke, muted_stroke);
        }

        vec![frame.into_geometry()]
    }
}

fn radial_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}
