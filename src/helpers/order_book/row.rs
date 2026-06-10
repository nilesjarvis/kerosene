mod cells;
mod marker;

use crate::message::Message;
use cells::{price_cell, size_cell, total_cell};
pub use marker::user_order_price_marker;

use iced::widget::button;
use iced::widget::container as container_style;
use iced::widget::{Space, container, row};
use iced::{Color, Element, Fill, Theme};

/// Fixed height of every order book row, in both display modes. The centered
/// layout's row-count math divides the available height by this constant, so
/// the rendered rows must occupy exactly this many pixels.
pub const BOOK_ROW_HEIGHT: f32 = 20.0;

#[derive(Debug, Clone, Copy)]
pub struct BookRowData {
    pub px: f64,
    pub sz: f64,
    pub cum: f64,
    pub has_user_order: bool,
    pub is_best: bool,
}

/// Render a single order book row with a depth bar background.
pub fn book_row(
    data: BookRowData,
    max_cum: f64,
    max_sz: f64,
    decimals: usize,
    is_bid: bool,
    reverse_side: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let px = data.px;
    let sz = data.sz;
    let cum = data.cum;
    let bar_pct = (cum / max_cum).clamp(0.0, 1.0) as f32;
    // Calculate heat from 0.0 to 1.0, slightly curved so medium orders are visible
    let heat = (sz / max_sz).clamp(0.0, 1.0).powf(0.5) as f32;

    // Minimum alpha for the underlying cumulative depth
    let base_alpha = 0.08;
    // Extra alpha added based on the size of the order at this level
    let heat_alpha = heat * 0.40;

    let color_start = move |theme: &Theme| {
        let mut c = if is_bid {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        c.a = base_alpha;
        c
    };

    let color_end = move |theme: &Theme| {
        let mut c = if is_bid {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        c.a = base_alpha + heat_alpha;
        c
    };

    // Keep size text readable regardless of heat; the background gradient
    // already carries the size signal, the text only echoes it subtly.
    let sz_pct = 0.65 + heat * 0.35;

    let price = price_cell(px, decimals, data.has_user_order, is_bid, data.is_best);
    let size = size_cell(sz, sz_pct);
    let total = total_cell(cum);
    let row_content = if reverse_side {
        row![total, size, price]
    } else {
        row![price, size, total]
    }
    .spacing(4)
    // The price cell is Fill-height (it hosts the best-level background);
    // center the shrink-height size/total cells so all three columns share
    // one baseline.
    .align_y(iced::Alignment::Center);

    let transparent = Color::TRANSPARENT;
    let row_element: Element<'static, Message> = container(row_content)
        .width(Fill)
        .height(BOOK_ROW_HEIGHT)
        .align_y(iced::alignment::Vertical::Center)
        .padding([0, 4])
        .style(move |theme: &Theme| {
            use iced::gradient;
            let gradient = if reverse_side {
                let end = bar_pct.clamp(0.0, 1.0);
                gradient::Linear::new(iced::Degrees(90.0))
                    .add_stop(0.0, color_end(theme))
                    .add_stop(end, color_start(theme))
                    .add_stop((end + 0.0001).min(1.0), transparent)
            } else {
                let start_point = 1.0 - bar_pct;
                let s_start = start_point.clamp(0.0, 1.0);
                gradient::Linear::new(iced::Degrees(90.0))
                    .add_stop(s_start.max(0.0001) - 0.0001, transparent)
                    .add_stop(s_start, color_start(theme))
                    .add_stop(1.0, color_end(theme))
            };

            // Smooth linear gradient based on Heatmap intensity
            container_style::Style {
                background: Some(gradient.into()),
                ..Default::default()
            }
        })
        .into();

    clickable_book_row(row_element, on_press)
}

/// Inert filler row used to keep the scrollable depth list at a constant
/// content height while the number of live levels fluctuates. Not clickable,
/// no hover affordance.
pub fn placeholder_book_row() -> Element<'static, Message> {
    container(Space::new())
        .width(Fill)
        .height(BOOK_ROW_HEIGHT)
        .into()
}

pub fn clickable_book_row(
    content: Element<'static, Message>,
    on_press: Message,
) -> Element<'static, Message> {
    button(content)
        .width(Fill)
        .padding(0)
        .style(|theme: &Theme, status| {
            let mut border_color = theme.palette().primary;
            border_color.a = match status {
                button::Status::Hovered => 0.42,
                button::Status::Pressed => 0.68,
                _ => 0.0,
            };

            button::Style {
                background: None,
                border: iced::Border {
                    radius: 2.0.into(),
                    width: 1.0,
                    color: border_color,
                },
                ..Default::default()
            }
        })
        .on_press(on_press)
        .into()
}
