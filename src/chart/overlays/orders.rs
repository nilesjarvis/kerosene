use super::TradingOverlayContext;
mod drawing;
mod segments;
mod visible;

use crate::chart::model::CandlestickChart;
use crate::chart::order_labels::{
    OrderLabelAnchor, order_label_position, order_label_position_slots,
    stack_order_label_positions_avoiding,
};

use drawing::{
    draw_order_label, draw_order_label_connector, draw_order_line, draw_order_price_badge,
};
use visible::collect_visible_orders;

// ---------------------------------------------------------------------------
// Order Overlays
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_active_order_lines<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if ctx.price_range <= 0.0
            || !ctx.price_range.is_finite()
            || ctx.chart_w <= 0.0
            || ctx.price_h <= 0.0
            || !ctx.chart_w.is_finite()
            || !ctx.price_h.is_finite()
        {
            return;
        }

        let visible_orders = collect_visible_orders(self, ctx);

        let reserved_ranges = self.order_label_reserved_ranges(ctx.price_h, ctx.price_to_y);
        let label_positions = order_label_position_slots(
            stack_order_label_positions_avoiding(
                visible_orders
                    .iter()
                    .map(|order| OrderLabelAnchor {
                        order_index: order.order_index,
                        order_y: order.order_y,
                        is_buy: order.is_buy,
                    })
                    .collect(),
                ctx.price_h,
                &reserved_ranges,
            ),
            self.active_orders.len(),
        );

        for visible_order in &visible_orders {
            if let Some(position) =
                order_label_position(&label_positions, visible_order.order_index)
            {
                draw_order_line(ctx, visible_order, position);
                draw_order_price_badge(ctx, visible_order);
            }
        }

        for visible_order in &visible_orders {
            if let Some(position) =
                order_label_position(&label_positions, visible_order.order_index)
            {
                draw_order_label_connector(ctx, visible_order, position);
            }
        }

        for visible_order in &visible_orders {
            if let Some(position) =
                order_label_position(&label_positions, visible_order.order_index)
            {
                draw_order_label(ctx, visible_order, position);
            }
        }
    }
}
