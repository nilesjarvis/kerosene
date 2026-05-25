use super::{OrderLabelAnchor, ReservedLabelRange};
use crate::chart::order_labels::ORDER_LABEL_STACK_GAP;

// ---------------------------------------------------------------------------
// Reserved Label Ranges
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReservedSide {
    Above,
    Below,
}

pub(super) fn preferred_side_of_reserved(
    anchor: OrderLabelAnchor,
    reserved_range: ReservedLabelRange,
) -> ReservedSide {
    let center_y = reserved_range.center();
    if anchor.order_y < center_y {
        ReservedSide::Above
    } else if anchor.order_y > center_y || anchor.is_buy {
        ReservedSide::Below
    } else {
        ReservedSide::Above
    }
}

pub(super) fn push_label_below_reserved(
    mut label_y: f32,
    label_half: f32,
    reserved_ranges: &[ReservedLabelRange],
) -> f32 {
    loop {
        let Some(range) = reserved_ranges
            .iter()
            .find(|range| label_overlaps_range(label_y, label_half, range))
        else {
            return label_y;
        };
        label_y = range.bottom + ORDER_LABEL_STACK_GAP + label_half;
    }
}

pub(super) fn push_label_above_reserved(
    mut label_y: f32,
    label_half: f32,
    reserved_ranges: &[ReservedLabelRange],
) -> f32 {
    loop {
        let Some(range) = reserved_ranges
            .iter()
            .rev()
            .find(|range| label_overlaps_range(label_y, label_half, range))
        else {
            return label_y;
        };
        label_y = range.top - ORDER_LABEL_STACK_GAP - label_half;
    }
}

fn label_overlaps_range(label_y: f32, label_half: f32, range: &ReservedLabelRange) -> bool {
    let label_top = label_y - label_half;
    let label_bottom = label_y + label_half;
    label_top < range.bottom + ORDER_LABEL_STACK_GAP
        && label_bottom > range.top - ORDER_LABEL_STACK_GAP
}
