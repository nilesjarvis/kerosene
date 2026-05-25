use super::super::model::CandlestickChart;
use super::super::state::ChartState;

mod anchors;
mod stacking;

use anchors::right_axis_badge_anchors;
#[cfg(test)]
pub(super) use stacking::{
    FixedBadgeSide, RIGHT_AXIS_BADGE_GAP, RIGHT_AXIS_BUY_ORDER_SORT_BASE,
    RIGHT_AXIS_POSITION_ENTRY_SORT_RANK, RIGHT_AXIS_SELL_ORDER_SORT_BASE, RightAxisBadgeAnchor,
    badge_bottom, badge_top,
};
pub(super) use stacking::{StackedRightAxisBadge, stack_right_axis_badge_positions};

// ---------------------------------------------------------------------------
// Right Axis Price Badge Layout
// ---------------------------------------------------------------------------

pub(in crate::chart) const RIGHT_AXIS_PRIMARY_BADGE_HEIGHT: f32 = 16.0;
pub(in crate::chart) const RIGHT_AXIS_SECONDARY_BADGE_HEIGHT: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::chart) enum RightAxisBadgeKind {
    CurrentPrice,
    QuickOrder,
    PositionEntry,
    PositionLiquidation,
    ActiveOrder(usize),
    HorizontalAnnotation(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct RightAxisBadgePosition {
    pub(in crate::chart) source_y: f32,
    pub(in crate::chart) badge_y: f32,
}

#[derive(Debug, Clone)]
pub(in crate::chart) struct RightAxisBadgeLayout {
    current_price: Option<RightAxisBadgePosition>,
    quick_order: Option<RightAxisBadgePosition>,
    position_entry: Option<RightAxisBadgePosition>,
    position_liquidation: Option<RightAxisBadgePosition>,
    active_orders: Vec<Option<RightAxisBadgePosition>>,
    horizontal_annotations: Vec<Option<RightAxisBadgePosition>>,
}

impl CandlestickChart {
    pub(in crate::chart) fn right_axis_badge_layout<PriceToY>(
        &self,
        state: &ChartState,
        price_h: f32,
        price_range: f64,
        price_to_y: &PriceToY,
    ) -> RightAxisBadgeLayout
    where
        PriceToY: Fn(f64) -> f32,
    {
        let mut layout =
            RightAxisBadgeLayout::empty(self.active_orders.len(), self.annotations.len());
        if price_range <= 0.0 || price_h <= 0.0 || !price_range.is_finite() || !price_h.is_finite()
        {
            return layout;
        }

        let anchors = right_axis_badge_anchors(self, state, price_h, price_to_y);

        for stacked in stack_right_axis_badge_positions(anchors, price_h) {
            layout.insert(stacked);
        }

        layout
    }
}

impl RightAxisBadgeLayout {
    fn empty(active_order_count: usize, annotation_count: usize) -> Self {
        Self {
            current_price: None,
            quick_order: None,
            position_entry: None,
            position_liquidation: None,
            active_orders: vec![None; active_order_count],
            horizontal_annotations: vec![None; annotation_count],
        }
    }

    pub(in crate::chart) fn position(
        &self,
        kind: RightAxisBadgeKind,
    ) -> Option<RightAxisBadgePosition> {
        match kind {
            RightAxisBadgeKind::CurrentPrice => self.current_price,
            RightAxisBadgeKind::QuickOrder => self.quick_order,
            RightAxisBadgeKind::PositionEntry => self.position_entry,
            RightAxisBadgeKind::PositionLiquidation => self.position_liquidation,
            RightAxisBadgeKind::ActiveOrder(index) => {
                self.active_orders.get(index).copied().flatten()
            }
            RightAxisBadgeKind::HorizontalAnnotation(index) => {
                self.horizontal_annotations.get(index).copied().flatten()
            }
        }
    }

    fn insert(&mut self, stacked: StackedRightAxisBadge) {
        let position = RightAxisBadgePosition {
            source_y: stacked.source_y,
            badge_y: stacked.badge_y,
        };
        match stacked.kind {
            RightAxisBadgeKind::CurrentPrice => self.current_price = Some(position),
            RightAxisBadgeKind::QuickOrder => self.quick_order = Some(position),
            RightAxisBadgeKind::PositionEntry => self.position_entry = Some(position),
            RightAxisBadgeKind::PositionLiquidation => {
                self.position_liquidation = Some(position);
            }
            RightAxisBadgeKind::ActiveOrder(index) => {
                if let Some(slot) = self.active_orders.get_mut(index) {
                    *slot = Some(position);
                }
            }
            RightAxisBadgeKind::HorizontalAnnotation(index) => {
                if let Some(slot) = self.horizontal_annotations.get_mut(index) {
                    *slot = Some(position);
                }
            }
        }
    }
}
