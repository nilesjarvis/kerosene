use crate::helpers::format_size;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{container, row, text};
use iced::{Color, Element, Fill, Theme};

/// Render a single order book row with a depth bar background.
pub fn book_row(
    px: f64,
    sz: f64,
    cum: f64,
    max_cum: f64,
    max_sz: f64,
    decimals: usize,
    is_bid: bool,
) -> Element<'static, Message> {
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

    let price_color = if is_bid {
        move |theme: &Theme| theme.palette().success
    } else {
        move |theme: &Theme| theme.palette().danger
    };

    // Text brightness also driven by heat (0.3 to 1.0)
    let sz_pct = heat.max(0.3);
    let size_color = move |theme: &Theme| Color {
        a: sz_pct,
        ..theme.palette().text
    };

    let total_color = move |theme: &Theme| theme.extended_palette().background.weak.text;

    let row_content = row![
        text(format!("{px:.decimals$}"))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |t: &Theme| text::Style {
                color: Some(price_color(t))
            })
            .width(Fill),
        text(format_size(sz))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |t: &Theme| text::Style {
                color: Some(size_color(t))
            })
            .width(Fill),
        text(format_size(cum))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |t: &Theme| text::Style {
                color: Some(total_color(t))
            })
            .width(Fill),
    ]
    .spacing(4);

    let transparent = Color::TRANSPARENT;
    container(row_content)
        .width(Fill)
        .padding([2, 4])
        .style(move |theme: &Theme| {
            use iced::gradient;
            let start_point = 1.0 - bar_pct;
            let s_start = start_point.clamp(0.0, 1.0);

            // Smooth linear gradient based on Heatmap intensity
            container_style::Style {
                background: Some(
                    gradient::Linear::new(iced::Degrees(90.0))
                        .add_stop(s_start.max(0.0001) - 0.0001, transparent)
                        .add_stop(s_start, color_start(theme))
                        .add_stop(1.0, color_end(theme))
                        .into(),
                ),
                ..Default::default()
            }
        })
        .into()
}
