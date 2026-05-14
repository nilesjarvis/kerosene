use crate::helpers::format_size;
use crate::message::Message;

use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme};

const USER_ORDER_MARKER_WIDTH: f32 = 10.0;
const USER_ORDER_MARKER_HEIGHT: f32 = 10.0;
const USER_ORDER_MARKER_RADIUS: f32 = 3.1;

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

    let row_content = row![
        price_cell(px, decimals, data.has_user_order, is_bid, price_color),
        text(format_size(sz))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |t: &Theme| text::Style {
                color: Some(size_color(t))
            })
            .width(Fill),
        total_cell(cum),
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

fn price_cell(
    px: f64,
    decimals: usize,
    has_user_order: bool,
    is_bid: bool,
    price_color: impl Fn(&Theme) -> Color + 'static,
) -> Element<'static, Message> {
    container(
        row![
            Space::new().width(Fill),
            user_order_price_marker(has_user_order.then_some(is_bid)),
            text(format!("{px:.decimals$}"))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .style(move |t: &Theme| text::Style {
                    color: Some(price_color(t))
                }),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .into()
}

fn total_cell(cum: f64) -> Element<'static, Message> {
    container(
        text(format_size(cum))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .align_x(iced::alignment::Horizontal::Right)
            .style(move |theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.weak.text),
            })
            .width(Fill),
    )
    .width(Fill)
    .into()
}

pub fn user_order_price_marker(user_order_side: Option<bool>) -> Element<'static, Message> {
    let Some(is_bid) = user_order_side else {
        return Space::new()
            .width(USER_ORDER_MARKER_WIDTH)
            .height(USER_ORDER_MARKER_HEIGHT)
            .into();
    };

    canvas(UserOrderPriceMarker { is_bid })
        .width(USER_ORDER_MARKER_WIDTH)
        .height(USER_ORDER_MARKER_HEIGHT)
        .into()
}

struct UserOrderPriceMarker {
    is_bid: bool,
}

impl canvas::Program<Message> for UserOrderPriceMarker {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let radius = USER_ORDER_MARKER_RADIUS.min((bounds.width.min(bounds.height) / 2.0).max(0.0));
        if radius <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let color = if self.is_bid {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        let path =
            canvas::Path::circle(Point::new(bounds.width / 2.0, bounds.height / 2.0), radius);
        frame.fill(&path, color);
        vec![frame.into_geometry()]
    }
}
