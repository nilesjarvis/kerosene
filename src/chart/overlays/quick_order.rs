use super::TradingOverlayContext;
use crate::chart::drawing::{
    AxisBadgeStyle, SegmentedHLineStyle, stroke_projected_segmented_hline_with_offset,
};
use crate::chart::model::CandlestickChart;
use crate::chart::price_badges::{
    RIGHT_AXIS_PRIMARY_BADGE_HEIGHT, RightAxisBadgeConnectorStyle, RightAxisBadgeKind,
    draw_stacked_right_axis_badge, right_axis_line_end_x,
};
use crate::helpers::format_price;
use iced::Color;

// ---------------------------------------------------------------------------
// Quick Order Limit Overlay
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_quick_order_limit_line<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        let Some(price) = self.quick_order_limit_price else {
            return;
        };
        if !price.is_finite() || price <= 0.0 || ctx.price_range <= 0.0 {
            return;
        }

        let y = (ctx.price_to_y)(price);
        if y < -10.0 || y > ctx.price_h + 10.0 {
            return;
        }

        let line_color = Color {
            a: 0.65,
            ..ctx.theme.palette().primary
        };
        let badge_kind = RightAxisBadgeKind::QuickOrder;
        let line_end_x = right_axis_line_end_x(ctx.right_axis_badges, badge_kind, ctx.chart_w);
        stroke_projected_segmented_hline_with_offset(
            ctx.frame,
            ctx.fisheye,
            line_end_x,
            y,
            SegmentedHLineStyle {
                segment_len: 8.0,
                gap_len: 4.0,
                offset: self.quick_order_line_phase,
                color: line_color,
                width: 1.5,
            },
        );
        draw_stacked_right_axis_badge(
            ctx.frame,
            ctx.right_axis_badges,
            badge_kind,
            ctx.chart_w,
            y,
            format_price(price),
            ctx.theme.palette().primary,
            AxisBadgeStyle {
                char_width: 6.5,
                padding_width: 8.0,
                height: RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
                text_size: 10.0,
                text_color: Color::BLACK,
            },
            RightAxisBadgeConnectorStyle::Segmented {
                style: SegmentedHLineStyle {
                    segment_len: 8.0,
                    gap_len: 4.0,
                    offset: self.quick_order_line_phase,
                    color: line_color,
                    width: 1.5,
                },
            },
            ctx.fisheye,
        );
    }
}
