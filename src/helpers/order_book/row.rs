mod cells;
mod marker;

use crate::message::Message;
use cells::{price_cell, size_cell, total_cell};
pub use marker::user_order_price_marker;

use iced::widget::button;
use iced::widget::container as container_style;
use iced::widget::{container, row};
use iced::{Color, Element, Fill, Theme};

#[derive(Debug, Clone, Copy)]
pub struct BookRowData {
    pub px: f64,
    pub sz: f64,
    pub cum: f64,
    pub has_user_order: bool,
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

    // Text brightness also driven by heat (0.3 to 1.0)
    let sz_pct = heat.max(0.3);

    let price = price_cell(px, decimals, data.has_user_order, is_bid);
    let size = size_cell(sz, sz_pct);
    let total = total_cell(cum);
    let row_content = if reverse_side {
        row![total, size, price]
    } else {
        row![price, size, total]
    }
    .spacing(4);

    let transparent = Color::TRANSPARENT;
    let row_element: Element<'static, Message> = container(row_content)
        .width(Fill)
        .padding([2, 4])
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
