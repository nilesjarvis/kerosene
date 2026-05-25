use super::{ORDER_LABEL_HEIGHT, ORDER_LABEL_STACK_GAP, ORDER_LABEL_STACK_MARGIN};

mod reserved;

use reserved::{
    ReservedSide, preferred_side_of_reserved, push_label_above_reserved, push_label_below_reserved,
};

// ---------------------------------------------------------------------------
// Order Label Stacking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct OrderLabelAnchor {
    pub(in crate::chart) order_index: usize,
    pub(in crate::chart) order_y: f32,
    pub(in crate::chart) is_buy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct OrderLabelPosition {
    pub(in crate::chart) order_index: usize,
    pub(in crate::chart) order_y: f32,
    pub(in crate::chart) label_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct ReservedLabelRange {
    top: f32,
    bottom: f32,
}

impl ReservedLabelRange {
    pub(in crate::chart) fn from_center(center_y: f32, height: f32) -> Self {
        Self {
            top: center_y - height * 0.5,
            bottom: center_y + height * 0.5,
        }
    }

    fn center(&self) -> f32 {
        (self.top + self.bottom) * 0.5
    }
}

pub(in crate::chart) fn stack_order_label_positions_avoiding(
    mut anchors: Vec<OrderLabelAnchor>,
    price_h: f32,
    reserved_ranges: &[ReservedLabelRange],
) -> Vec<OrderLabelPosition> {
    if anchors.is_empty() {
        return Vec::new();
    }

    let mut reserved_ranges = reserved_ranges.to_vec();
    reserved_ranges.sort_by(|a, b| a.top.total_cmp(&b.top));

    anchors.sort_by(|a, b| {
        a.order_y
            .total_cmp(&b.order_y)
            .then_with(|| a.order_index.cmp(&b.order_index))
    });

    let min_y = ORDER_LABEL_HEIGHT * 0.5 + ORDER_LABEL_STACK_MARGIN;
    let max_y = (price_h - ORDER_LABEL_HEIGHT * 0.5 - ORDER_LABEL_STACK_MARGIN).max(min_y);
    if reserved_ranges.len() == 1 {
        return stack_order_label_positions_around_reserved(
            anchors,
            min_y,
            max_y,
            reserved_ranges[0],
        );
    }

    let step = ORDER_LABEL_HEIGHT + ORDER_LABEL_STACK_GAP;
    let label_half = ORDER_LABEL_HEIGHT * 0.5;
    let mut positions = Vec::with_capacity(anchors.len());
    let mut next_y = min_y;

    for anchor in anchors {
        let desired_y = anchor.order_y.clamp(min_y, max_y);
        let label_y =
            push_label_below_reserved(desired_y.max(next_y), label_half, &reserved_ranges);
        positions.push(OrderLabelPosition {
            order_index: anchor.order_index,
            order_y: anchor.order_y,
            label_y,
        });
        next_y = label_y + step;
    }

    if positions
        .last()
        .is_some_and(|position| position.label_y > max_y)
    {
        let mut next_y = max_y;
        for position in positions.iter_mut().rev() {
            position.label_y = position.label_y.min(next_y);
            position.label_y =
                push_label_above_reserved(position.label_y, label_half, &reserved_ranges);
            next_y = position.label_y - step;
        }

        if let Some(first) = positions.first()
            && first.label_y < min_y
        {
            let shift = min_y - first.label_y;
            for position in &mut positions {
                position.label_y += shift;
            }
        }
    }

    positions
}

fn stack_order_label_positions_around_reserved(
    anchors: Vec<OrderLabelAnchor>,
    min_y: f32,
    max_y: f32,
    reserved_range: ReservedLabelRange,
) -> Vec<OrderLabelPosition> {
    let label_half = ORDER_LABEL_HEIGHT * 0.5;
    let above_max_y = (reserved_range.top - ORDER_LABEL_STACK_GAP - label_half).max(min_y);
    let below_min_y = (reserved_range.bottom + ORDER_LABEL_STACK_GAP + label_half).min(max_y);
    let mut above = Vec::new();
    let mut below = Vec::new();

    for anchor in anchors {
        match preferred_side_of_reserved(anchor, reserved_range) {
            ReservedSide::Above => above.push(anchor),
            ReservedSide::Below => below.push(anchor),
        }
    }

    let mut positions = stack_order_label_positions_in_band(above, min_y, above_max_y);
    positions.extend(stack_order_label_positions_in_band(
        below,
        below_min_y,
        max_y,
    ));
    positions.sort_by_key(|position| position.order_index);
    positions
}

fn stack_order_label_positions_in_band(
    anchors: Vec<OrderLabelAnchor>,
    min_y: f32,
    max_y: f32,
) -> Vec<OrderLabelPosition> {
    if anchors.is_empty() {
        return Vec::new();
    }

    let step = ORDER_LABEL_HEIGHT + ORDER_LABEL_STACK_GAP;
    let mut positions = Vec::with_capacity(anchors.len());
    let mut next_y = min_y;

    for anchor in anchors {
        let desired_y = anchor.order_y.clamp(min_y, max_y);
        let label_y = desired_y.max(next_y);
        positions.push(OrderLabelPosition {
            order_index: anchor.order_index,
            order_y: anchor.order_y,
            label_y,
        });
        next_y = label_y + step;
    }

    if positions
        .last()
        .is_some_and(|position| position.label_y > max_y)
    {
        let mut next_y = max_y;
        for position in positions.iter_mut().rev() {
            position.label_y = position.label_y.min(next_y);
            next_y = position.label_y - step;
        }

        if let Some(first) = positions.first()
            && first.label_y < min_y
        {
            let shift = min_y - first.label_y;
            for position in &mut positions {
                position.label_y += shift;
            }
        }
    }

    positions
}
