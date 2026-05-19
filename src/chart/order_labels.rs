use super::{CandlestickChart, OrderOverlay};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OrderLabelAnchor {
    pub(super) order_index: usize,
    pub(super) order_y: f32,
    pub(super) is_buy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OrderLabelPosition {
    pub(super) order_index: usize,
    pub(super) order_y: f32,
    pub(super) label_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ReservedLabelRange {
    top: f32,
    bottom: f32,
}

impl ReservedLabelRange {
    pub(super) fn from_center(center_y: f32, height: f32) -> Self {
        Self {
            top: center_y - height * 0.5,
            bottom: center_y + height * 0.5,
        }
    }

    fn center(&self) -> f32 {
        (self.top + self.bottom) * 0.5
    }
}

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

pub(super) fn stack_order_label_positions_avoiding(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReservedSide {
    Above,
    Below,
}

fn preferred_side_of_reserved(
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

fn push_label_below_reserved(
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

fn push_label_above_reserved(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_label_stack_separates_nearby_orders() {
        let positions = stack_order_label_positions_avoiding(
            vec![
                OrderLabelAnchor {
                    order_index: 0,
                    order_y: 40.0,
                    is_buy: true,
                },
                OrderLabelAnchor {
                    order_index: 1,
                    order_y: 42.0,
                    is_buy: true,
                },
                OrderLabelAnchor {
                    order_index: 2,
                    order_y: 43.0,
                    is_buy: true,
                },
            ],
            240.0,
            &[],
        );

        assert_eq!(positions[0].label_y, 40.0);
        assert_eq!(positions[1].label_y, 54.0);
        assert_eq!(positions[2].label_y, 68.0);
    }

    #[test]
    fn order_label_stack_stays_inside_available_height_when_possible() {
        let positions = stack_order_label_positions_avoiding(
            vec![
                OrderLabelAnchor {
                    order_index: 0,
                    order_y: 96.0,
                    is_buy: true,
                },
                OrderLabelAnchor {
                    order_index: 1,
                    order_y: 98.0,
                    is_buy: true,
                },
            ],
            100.0,
            &[],
        );

        assert_eq!(positions[0].label_y, 78.0);
        assert_eq!(positions[1].label_y, 92.0);
    }

    #[test]
    fn order_label_stack_avoids_reserved_position_label() {
        let positions = stack_order_label_positions_avoiding(
            vec![OrderLabelAnchor {
                order_index: 0,
                order_y: 40.0,
                is_buy: true,
            }],
            240.0,
            &[ReservedLabelRange::from_center(40.0, POSITION_LABEL_HEIGHT)],
        );

        assert_eq!(positions[0].label_y, 57.0);
    }

    #[test]
    fn order_label_stack_keeps_asks_above_position_label() {
        let positions = stack_order_label_positions_avoiding(
            vec![OrderLabelAnchor {
                order_index: 0,
                order_y: 40.0,
                is_buy: false,
            }],
            240.0,
            &[ReservedLabelRange::from_center(40.0, POSITION_LABEL_HEIGHT)],
        );

        assert_eq!(positions[0].label_y, 23.0);
    }

    #[test]
    fn order_label_stack_keeps_bid_labels_below_position_label() {
        let positions = stack_order_label_positions_avoiding(
            vec![
                OrderLabelAnchor {
                    order_index: 0,
                    order_y: 40.0,
                    is_buy: true,
                },
                OrderLabelAnchor {
                    order_index: 1,
                    order_y: 41.0,
                    is_buy: true,
                },
            ],
            240.0,
            &[ReservedLabelRange::from_center(40.0, POSITION_LABEL_HEIGHT)],
        );

        assert_eq!(positions[0].label_y, 57.0);
        assert_eq!(positions[1].label_y, 71.0);
    }
}
