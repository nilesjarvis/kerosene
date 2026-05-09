use super::geometry::ORDER_HIT_TOLERANCE;
use super::{CandlestickChart, ChartState, OrderOverlay};
use iced::Point;

pub(super) struct OrderLineHit<'a> {
    pub(super) order: &'a OrderOverlay,
}

impl CandlestickChart {
    pub(super) fn hit_test_order_line<'a>(
        &'a self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<OrderLineHit<'a>> {
        if pos.x >= chart_w || pos.y >= chart_h || self.active_orders.is_empty() {
            return None;
        }

        let (price_hi, price_range, price_h) =
            self.visible_price_params(state, chart_w, chart_h)?;

        self.active_orders.iter().find_map(|order| {
            let order_y = self.price_to_y_with(order.limit_px, price_hi, price_range, price_h);
            ((pos.y - order_y).abs() < ORDER_HIT_TOLERANCE
                && order_y >= -10.0
                && order_y <= price_h + 10.0)
                .then_some(OrderLineHit { order })
        })
    }

    pub(super) fn order_cancel_x_range(order: &OrderOverlay) -> (f32, f32) {
        let side_str = if order.is_buy { "BUY" } else { "SELL" };
        let side_label = format!("{side_str} {:.4}", order.sz);
        let side_bg_w = side_label.len() as f32 * 5.5 + 8.0;
        let cancel_x = 4.0 + side_bg_w + 3.0;
        (cancel_x, cancel_x + 12.0)
    }
}
