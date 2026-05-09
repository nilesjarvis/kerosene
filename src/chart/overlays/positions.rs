use super::TradingOverlayContext;
use crate::chart::drawing::{AxisBadgeStyle, fill_right_axis_badge, stroke_segmented_hline};
use crate::chart::model::CandlestickChart;
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

// ---------------------------------------------------------------------------
// Position Overlays
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_active_position_lines<PriceToY>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if let Some(pos_overlay) = &self.active_position
            && ctx.price_range > 0.0
        {
            let entry_y = (ctx.price_to_y)(pos_overlay.entry_px);
            if entry_y >= -10.0 && entry_y <= ctx.price_h + 10.0 {
                let is_long = pos_overlay.szi > 0.0;
                let pos_color = if is_long {
                    Color {
                        a: 0.50,
                        ..ctx.theme.palette().success
                    }
                } else {
                    Color {
                        a: 0.50,
                        ..ctx.theme.palette().danger
                    }
                };
                let pos_color_solid = if is_long {
                    ctx.theme.palette().success
                } else {
                    ctx.theme.palette().danger
                };

                stroke_segmented_hline(ctx.frame, ctx.chart_w, entry_y, 6.0, 4.0, pos_color, 1.0);
                fill_right_axis_badge(
                    ctx.frame,
                    ctx.chart_w,
                    entry_y,
                    format_price(pos_overlay.entry_px),
                    pos_color_solid,
                    AxisBadgeStyle {
                        char_width: 6.5,
                        padding_width: 8.0,
                        height: 16.0,
                        text_size: 10.0,
                        text_color: Color::BLACK,
                    },
                );

                let side_label = if is_long { "LONG" } else { "SHORT" };
                let side_bg_w = 40.0_f32;
                let side_bg_h = 14.0_f32;
                ctx.frame.fill_rectangle(
                    Point::new(4.0, entry_y - side_bg_h * 0.5),
                    Size::new(side_bg_w, side_bg_h),
                    Color {
                        a: 0.6,
                        ..ctx.theme.extended_palette().background.strong.color
                    },
                );
                ctx.frame.fill_text(canvas::Text {
                    content: side_label.to_string(),
                    position: Point::new(6.0, entry_y),
                    color: pos_color_solid,
                    size: iced::Pixels(9.0),
                    align_x: alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Center,
                    font: iced::Font::MONOSPACE,
                    ..canvas::Text::default()
                });
            }

            if let Some(liq_px) = pos_overlay.liquidation_px {
                let liq_y = (ctx.price_to_y)(liq_px);
                if liq_y >= -10.0 && liq_y <= ctx.price_h + 10.0 {
                    let liq_color = ctx.theme.palette().primary;
                    let liq_color_dim = Color {
                        a: 0.55,
                        ..ctx.theme.palette().primary
                    };

                    stroke_segmented_hline(
                        ctx.frame,
                        ctx.chart_w,
                        liq_y,
                        2.0,
                        3.0,
                        liq_color_dim,
                        1.0,
                    );
                    fill_right_axis_badge(
                        ctx.frame,
                        ctx.chart_w,
                        liq_y,
                        format!("Liq {}", format_price(liq_px)),
                        liq_color,
                        AxisBadgeStyle {
                            char_width: 6.2,
                            padding_width: 9.0,
                            height: 14.0,
                            text_size: 8.5,
                            text_color: Color::BLACK,
                        },
                    );
                }
            }
        }
    }
}
