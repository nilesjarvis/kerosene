use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, format_size};
use crate::message::Message;
use crate::signing::{ChaseOrder, ChasePendingOp};
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
        if !self.chase_orders.is_empty() {
            header = header.push(stop_all_button());
        }

        let mut rows = Column::new().spacing(4);
        if self.chase_orders.is_empty() {
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
                rows = rows.push(advanced_order_row(chase, &theme, self.spinner_phase));
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

fn advanced_order_row(
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
    let status_color = if chase.stop_requested {
        theme.palette().danger
    } else if chase.pending_op.is_some()
        || chase.pending_best_price.is_some()
        || chase.current_oid.is_none()
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
    let oid = chase
        .current_oid
        .map(|oid| format!("#{oid}"))
        .unwrap_or_else(|| "-".to_string());
    let reduce_only = if chase.reduce_only { " | RO" } else { "" };
    let meta = format!("{oid} | {} reprices{reduce_only}", chase.reprice_count);

    container(
        row![
            spinning_gear(spinner_phase, 13, theme.palette().primary),
            text(side).size(10).color(side_color),
            text(chase.coin.clone()).size(12).width(Fill),
            text(format!("{} @ {price}", format_size(chase.remaining_size))).size(11),
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

fn chase_status(chase: &ChaseOrder) -> &'static str {
    if chase.stop_requested {
        return "Stopping";
    }
    if chase.missing_open_order_refresh_requested {
        return "Checking";
    }
    match chase.pending_op {
        Some(ChasePendingOp::Place) => "Placing",
        Some(ChasePendingOp::Modify { .. }) => "Modifying",
        Some(ChasePendingOp::Cancel { .. }) => "Canceling",
        None if chase.pending_best_price.is_some() => "Queued",
        None if chase.current_oid.is_none() => "Starting",
        None => "Resting",
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

fn stop_all_button() -> Element<'static, Message> {
    button(text("Stop All").size(10).center())
        .on_press(Message::StopAllChases)
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
