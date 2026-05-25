use super::{CandlestickChart, OrderOverlay};

mod stacking;
pub(super) use stacking::{
    OrderLabelAnchor, OrderLabelPosition, ReservedLabelRange, stack_order_label_positions_avoiding,
};

// ---------------------------------------------------------------------------
// Order Label Layout
// ---------------------------------------------------------------------------

pub(super) const ORDER_LABEL_X: f32 = 4.0;
pub(super) const ORDER_LABEL_TEXT_X: f32 = 6.0;
pub(super) const ORDER_LABEL_HEIGHT: f32 = 12.0;
pub(super) const ORDER_LABEL_CHAR_WIDTH: f32 = 5.5;
pub(super) const ORDER_LABEL_PADDING_WIDTH: f32 = 8.0;
pub(super) const ORDER_CANCEL_GAP: f32 = 3.0;
pub(super) const ORDER_CANCEL_WIDTH: f32 = 12.0;
pub(super) const ORDER_LABEL_STACK_GAP: f32 = 2.0;
pub(super) const ORDER_LABEL_STACK_MARGIN: f32 = 2.0;
pub(super) const ORDER_LABEL_CONNECTOR_SPAN: f32 = 24.0;
pub(super) const ORDER_LABEL_HIT_MAX_X: f32 = 220.0;
pub(super) const POSITION_LABEL_HEIGHT: f32 = 18.0;

impl CandlestickChart {
    pub(super) fn order_label_reserved_ranges<PriceToY>(
        &self,
        price_h: f32,
        price_to_y: &PriceToY,
    ) -> Vec<ReservedLabelRange>
    where
        PriceToY: Fn(f64) -> f32,
    {
        if self.obscure_position_prices {
            return Vec::new();
        }

        let Some(position) = &self.active_position else {
            return Vec::new();
        };

        let entry_y = price_to_y(position.entry_px);
        if entry_y >= -10.0 && entry_y <= price_h + 10.0 {
            vec![ReservedLabelRange::from_center(
                entry_y,
                POSITION_LABEL_HEIGHT,
            )]
        } else {
            Vec::new()
        }
    }
}

pub(super) fn order_side_label(order: &OrderOverlay) -> String {
    let side_str = if order.is_buy { "BUY" } else { "SELL" };
    format!("{side_str} {:.4}", order.sz)
}

pub(super) fn order_side_label_width(order: &OrderOverlay) -> f32 {
    order_side_label_width_for_label(&order_side_label(order))
}

pub(super) fn order_side_label_width_for_label(label: &str) -> f32 {
    label.len() as f32 * ORDER_LABEL_CHAR_WIDTH + ORDER_LABEL_PADDING_WIDTH
}

pub(super) fn order_cancel_x_range(order: &OrderOverlay) -> (f32, f32) {
    let cancel_x = ORDER_LABEL_X + order_side_label_width(order) + ORDER_CANCEL_GAP;
    (cancel_x, cancel_x + ORDER_CANCEL_WIDTH)
}

pub(super) fn order_label_x_range(order: &OrderOverlay) -> (f32, f32) {
    let (_, cancel_end) = order_cancel_x_range(order);
    (ORDER_LABEL_X, cancel_end)
}

pub(super) fn order_label_y_range(label_y: f32) -> (f32, f32) {
    (
        label_y - ORDER_LABEL_HEIGHT * 0.5,
        label_y + ORDER_LABEL_HEIGHT * 0.5,
    )
}

pub(super) fn order_label_position_slots(
    positions: Vec<OrderLabelPosition>,
    order_count: usize,
) -> Vec<Option<OrderLabelPosition>> {
    let mut slots = vec![None; order_count];
    for position in positions {
        if let Some(slot) = slots.get_mut(position.order_index) {
            *slot = Some(position);
        }
    }
    slots
}

pub(super) fn order_label_position(
    positions: &[Option<OrderLabelPosition>],
    order_index: usize,
) -> Option<OrderLabelPosition> {
    positions.get(order_index).copied().flatten()
}

#[cfg(test)]
mod tests;
