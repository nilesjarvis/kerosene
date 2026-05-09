use super::TradingOverlayContext;
use crate::chart::drawing::{AxisBadgeStyle, fill_right_axis_badge, stroke_segmented_hline};
use crate::chart::model::CandlestickChart;
use crate::chart::state::DragKind;
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

// ---------------------------------------------------------------------------
// Order Overlays
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_active_order_lines<PriceToY>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.price_range <= 0.0 {
            return;
        }

        let dragging_oid = match ctx.state.drag {
            Some(DragKind::MoveOrder { oid }) => Some(oid),
            _ => None,
        };

        for order in &self.active_orders {
            let is_dragging = dragging_oid == Some(order.oid);
            let display_px = if is_dragging {
                ctx.state.drag_order_new_price.unwrap_or(order.limit_px)
            } else {
                order.limit_px
            };

            let order_y = (ctx.price_to_y)(display_px);
            if order_y < -10.0 || order_y > ctx.price_h + 10.0 {
                continue;
            }

            let (order_color, order_color_solid, line_width) = if is_dragging {
                if order.is_buy {
                    (
                        Color {
                            a: 0.60,
                            ..ctx.theme.palette().success
                        },
                        ctx.theme.palette().success,
                        2.0,
                    )
                } else {
                    (
                        Color {
                            a: 0.60,
                            ..ctx.theme.palette().danger
                        },
                        ctx.theme.palette().danger,
                        2.0,
                    )
                }
            } else if order.is_buy {
                (
                    Color {
                        a: 0.35,
                        ..ctx.theme.palette().success
                    },
                    ctx.theme.palette().success,
                    1.0,
                )
            } else {
                (
                    Color {
                        a: 0.35,
                        ..ctx.theme.palette().danger
                    },
                    ctx.theme.palette().danger,
                    1.0,
                )
            };

            stroke_segmented_hline(
                ctx.frame,
                ctx.chart_w,
                order_y,
                3.0,
                5.0,
                order_color,
                line_width,
            );
            fill_right_axis_badge(
                ctx.frame,
                ctx.chart_w,
                order_y,
                format_price(display_px),
                order_color_solid,
                AxisBadgeStyle {
                    char_width: 6.5,
                    padding_width: 8.0,
                    height: 14.0,
                    text_size: 9.0,
                    text_color: Color::BLACK,
                },
            );

            let side_str = if order.is_buy { "BUY" } else { "SELL" };
            let side_label = format!("{side_str} {:.4}", order.sz);
            let side_bg_w = side_label.len() as f32 * 5.5 + 8.0;
            let side_bg_h = 12.0_f32;
            ctx.frame.fill_rectangle(
                Point::new(4.0, order_y - side_bg_h * 0.5),
                Size::new(side_bg_w, side_bg_h),
                Color {
                    a: 0.6,
                    ..ctx.theme.extended_palette().background.strong.color
                },
            );
            ctx.frame.fill_text(canvas::Text {
                content: side_label,
                position: Point::new(6.0, order_y),
                color: order_color_solid,
                size: iced::Pixels(8.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Center,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });

            let cancel_x = 4.0 + side_bg_w + 3.0;
            let cancel_bg_w = 12.0_f32;
            ctx.frame.fill_rectangle(
                Point::new(cancel_x, order_y - side_bg_h * 0.5),
                Size::new(cancel_bg_w, side_bg_h),
                Color {
                    a: 0.5,
                    ..ctx.theme.palette().danger
                },
            );
            ctx.frame.fill_text(canvas::Text {
                content: "x".to_string(),
                position: Point::new(cancel_x + cancel_bg_w * 0.5, order_y),
                color: Color::WHITE,
                size: iced::Pixels(8.0),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }
}
