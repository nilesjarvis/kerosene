use super::*;

mod basic;
mod reserved;

fn anchor(order_index: usize, order_y: f32, is_buy: bool) -> OrderLabelAnchor {
    OrderLabelAnchor {
        order_index,
        order_y,
        is_buy,
    }
}

fn position_label_range(center_y: f32) -> ReservedLabelRange {
    ReservedLabelRange::from_center(center_y, POSITION_LABEL_HEIGHT)
}
