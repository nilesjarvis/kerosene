use super::fisheye::ChartFisheye;
use super::geometry::ORDER_HIT_TOLERANCE;
use super::order_labels::{
    ORDER_CANCEL_HOVER_RADIUS, ORDER_LABEL_HIT_MAX_X, ORDER_LABEL_X, OrderLabelAnchor,
    order_cancel_x_range, order_label_position, order_label_position_slots, order_label_x_range,
    order_label_y_range, stack_order_label_positions_avoiding,
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
    label_y: Option<f32>,
}

impl OrderLineHit<'_> {
    pub(super) fn is_label_hit(&self) -> bool {
        self.target == OrderHitTarget::Label
    }

    pub(super) fn is_cancel_hit(&self, pos: Point) -> bool {
        if !self.is_label_hit() {
            return false;
        }
        let Some(label_y) = self.label_y else {
            return false;
        };
        order_cancel_contains(self.order, pos, label_y)
    }
}

impl CandlestickChart {
    #[cfg(test)]
    pub(super) fn hit_test_order_line<'a>(
        &'a self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<OrderLineHit<'a>> {
        self.hit_test_order_line_at(state, pos, pos, chart_w, chart_h, ChartFisheye::disabled())
    }

    pub(super) fn hit_test_order_line_at<'a>(
        &'a self,
        state: &ChartState,
        source_pos: Point,
        visual_pos: Point,
        chart_w: f32,
        chart_h: f32,
        fisheye: ChartFisheye,
    ) -> Option<OrderLineHit<'a>> {
        if source_pos.x >= chart_w
            || source_pos.y >= chart_h
            || self.active_orders.is_empty()
            || chart_w <= 0.0
            || chart_h <= 0.0
            || !chart_w.is_finite()
            || !chart_h.is_finite()
            || !source_pos.x.is_finite()
            || !source_pos.y.is_finite()
            || !visual_pos.x.is_finite()
            || !visual_pos.y.is_finite()
        {
            return None;
        }

        let (price_hi, price_range, price_h) =
            self.visible_price_params(state, chart_w, chart_h)?;
        let price_to_y = |price| self.price_to_y_with(price, price_hi, price_range, price_h);

        let label_positions = (visual_pos.x <= ORDER_LABEL_HIT_MAX_X).then(|| {
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
                    let visual_label_y = visual_order_label_y(position.label_y, fisheye);
                    let (label_x, label_end_x) = order_label_x_range(order);
                    let (label_y, label_end_y) = order_label_y_range(visual_label_y);
                    let label_contains = visual_pos.x >= label_x
                        && visual_pos.x <= label_end_x
                        && visual_pos.y >= label_y
                        && visual_pos.y <= label_end_y;
                    if label_contains || order_cancel_contains(order, visual_pos, visual_label_y) {
                        return Some(OrderLineHit {
                            order,
                            target: OrderHitTarget::Label,
                            label_y: Some(visual_label_y),
                        });
                    }
                }

                ((source_pos.y - order_y).abs() < ORDER_HIT_TOLERANCE).then_some(OrderLineHit {
                    order,
                    target: OrderHitTarget::Line,
                    label_y: None,
                })
            })
    }

    #[cfg(test)]
    pub(super) fn hit_test_order_cancel(
        &self,
        state: &ChartState,
        pos: Point,
        chart_w: f32,
        chart_h: f32,
    ) -> Option<u64> {
        let hit = self.hit_test_order_line(state, pos, chart_w, chart_h)?;
        hit.is_cancel_hit(pos).then_some(hit.order.oid)
    }

    pub(super) fn hit_test_order_cancel_at(
        &self,
        state: &ChartState,
        source_pos: Point,
        visual_pos: Point,
        chart_w: f32,
        chart_h: f32,
        fisheye: ChartFisheye,
    ) -> Option<u64> {
        let hit =
            self.hit_test_order_line_at(state, source_pos, visual_pos, chart_w, chart_h, fisheye)?;
        hit.is_cancel_hit(visual_pos).then_some(hit.order.oid)
    }
}

fn visual_order_label_y(label_y: f32, fisheye: ChartFisheye) -> f32 {
    fisheye.project(Point::new(ORDER_LABEL_X, label_y)).y
}

fn order_cancel_contains(order: &OrderOverlay, pos: Point, label_y: f32) -> bool {
    let (cancel_x, cancel_end) = order_cancel_x_range(order);
    let (label_y_min, label_y_max) = order_label_y_range(label_y);
    let in_rect =
        pos.x >= cancel_x && pos.x <= cancel_end && pos.y >= label_y_min && pos.y <= label_y_max;
    if in_rect {
        return true;
    }

    let center_x = (cancel_x + cancel_end) * 0.5;
    let dx = pos.x - center_x;
    let dy = pos.y - label_y;
    dx * dx + dy * dy <= ORDER_CANCEL_HOVER_RADIUS * ORDER_CANCEL_HOVER_RADIUS
}
