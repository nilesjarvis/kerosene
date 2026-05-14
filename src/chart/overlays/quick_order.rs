use super::TradingOverlayContext;
use crate::chart::drawing::{
    AxisBadgeStyle, SegmentedHLineStyle, fill_right_axis_badge, stroke_segmented_hline_with_offset,
};
use crate::chart::model::CandlestickChart;
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
        stroke_segmented_hline_with_offset(
            ctx.frame,
            ctx.chart_w,
            y,
            SegmentedHLineStyle {
                segment_len: 8.0,
                gap_len: 4.0,
                offset: self.quick_order_line_phase,
                color: line_color,
                width: 1.5,
            },
        );
        fill_right_axis_badge(
            ctx.frame,
            ctx.chart_w,
            y,
            format_price(price),
            ctx.theme.palette().primary,
            AxisBadgeStyle {
                char_width: 6.5,
                padding_width: 8.0,
                height: 16.0,
                text_size: 10.0,
                text_color: Color::BLACK,
            },
        );
    }
}
