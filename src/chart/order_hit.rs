use super::geometry::ORDER_HIT_TOLERANCE;
use super::order_labels::{
    ORDER_LABEL_HIT_MAX_X, OrderLabelAnchor, order_cancel_x_range, order_label_position,
    order_label_position_slots, order_label_x_range, order_label_y_range,
    stack_order_label_positions_avoiding,
};
use super::{CandlestickChart, ChartState, OrderOverlay};
use iced::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderHitTarget {
    Line,
    Label,
}

pub(super) struct OrderLineHit<'a> {
    pub(super) order: &'a OrderOverlay,
    target: OrderHitTarget,
}

impl OrderLineHit<'_> {
    pub(super) fn is_label_hit(&self) -> bool {
        self.target == OrderHitTarget::Label
    }
}

impl CandlestickChart {
    pub(super) fn hit_test_order_line<'a>(
        &'a self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<OrderLineHit<'a>> {
        if pos.x >= chart_w
            || pos.y >= chart_h
            || self.active_orders.is_empty()
            || chart_w <= 0.0
            || chart_h <= 0.0
            || !chart_w.is_finite()
            || !chart_h.is_finite()
            || !pos.x.is_finite()
            || !pos.y.is_finite()
        {
            return None;
        }

        let (price_hi, price_range, price_h) =
            self.visible_price_params(state, chart_w, chart_h)?;
        let price_to_y = |price| self.price_to_y_with(price, price_hi, price_range, price_h);

        let label_positions = (pos.x <= ORDER_LABEL_HIT_MAX_X).then(|| {
            let reserved_ranges = self.order_label_reserved_ranges(price_h, &price_to_y);
            order_label_position_slots(
                stack_order_label_positions_avoiding(
                    self.active_orders
                        .iter()
                        .enumerate()
                        .filter(|(_, order)| order.pending_state.is_none())
                        .filter_map(|(order_index, order)| {
                            let order_y = price_to_y(order.limit_px);
                            (order_y >= -10.0 && order_y <= price_h + 10.0).then_some(
                                OrderLabelAnchor {
                                    order_index,
                                    order_y,
                                    is_buy: order.is_buy,
                                },
                            )
                        })
                        .collect(),
                    price_h,
                    &reserved_ranges,
                ),
                self.active_orders.len(),
            )
        });

        self.active_orders
            .iter()
            .enumerate()
            .find_map(|(order_index, order)| {
                if order.pending_state.is_some() {
                    return None;
                }

                let order_y = price_to_y(order.limit_px);
                if order_y < -10.0 || order_y > price_h + 10.0 || !order_y.is_finite() {
                    return None;
                }

                if let Some(position) = label_positions
                    .as_deref()
                    .and_then(|positions| order_label_position(positions, order_index))
                {
                    let (label_x, label_end_x) = order_label_x_range(order);
                    let (label_y, label_end_y) = order_label_y_range(position.label_y);
                    if pos.x >= label_x
                        && pos.x <= label_end_x
                        && pos.y >= label_y
                        && pos.y <= label_end_y
                    {
                        return Some(OrderLineHit {
                            order,
                            target: OrderHitTarget::Label,
                        });
                    }
                }

                ((pos.y - order_y).abs() < ORDER_HIT_TOLERANCE).then_some(OrderLineHit {
                    order,
                    target: OrderHitTarget::Line,
                })
            })
    }

    pub(super) fn order_cancel_x_range(order: &OrderOverlay) -> (f32, f32) {
        order_cancel_x_range(order)
    }
}
