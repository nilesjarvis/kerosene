use crate::helpers::format_size;
use crate::message::Message;

use iced::widget::button;
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

fn price_cell(
    px: f64,
    decimals: usize,
    has_user_order: bool,
    is_bid: bool,
) -> Element<'static, Message> {
    container(
        row![
            Space::new().width(Fill),
            user_order_price_marker(has_user_order.then_some(is_bid)),
            text(format!("{px:.decimals$}"))
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .style(move |t: &Theme| text::Style {
                    color: Some(if is_bid {
                        t.palette().success
                    } else {
                        t.palette().danger
                    })
                }),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .into()
}

fn size_cell(sz: f64, alpha: f32) -> Element<'static, Message> {
    text(format_size(sz))
        .size(12)
        .font(crate::app_fonts::monospace_font())
        .align_x(iced::alignment::Horizontal::Right)
        .style(move |theme: &Theme| text::Style {
            color: Some(Color {
                a: alpha,
                ..theme.palette().text
            }),
        })
        .width(Fill)
        .into()
}

fn total_cell(cum: f64) -> Element<'static, Message> {
    container(
        text(format_size(cum))
            .size(12)
            .font(crate::app_fonts::monospace_font())
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
