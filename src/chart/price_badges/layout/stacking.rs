mod model;

use super::RightAxisBadgeKind;

use model::RIGHT_AXIS_BADGE_MARGIN;
pub(in crate::chart::price_badges) use model::{
    FixedBadgeSide, RIGHT_AXIS_BADGE_GAP, RIGHT_AXIS_BUY_ORDER_SORT_BASE,
    RIGHT_AXIS_POSITION_ENTRY_SORT_RANK, RIGHT_AXIS_SELL_ORDER_SORT_BASE, RightAxisBadgeAnchor,
    StackedRightAxisBadge, badge_bottom, badge_top, push_visible_badge,
};
pub(super) use model::{
    RIGHT_AXIS_ANNOTATION_SORT_BASE, RIGHT_AXIS_CURRENT_PRICE_SORT_RANK,
    RIGHT_AXIS_LIQUIDATION_SORT_RANK, RIGHT_AXIS_QUICK_ORDER_SORT_RANK,
};

// ---------------------------------------------------------------------------
// Badge Stacking
// ---------------------------------------------------------------------------

pub(in crate::chart::price_badges) fn stack_right_axis_badge_positions(
    mut anchors: Vec<RightAxisBadgeAnchor>,
    price_h: f32,
) -> Vec<StackedRightAxisBadge> {
    anchors.retain(|anchor| {
        anchor.source_y.is_finite() && anchor.height > 0.0 && anchor.height.is_finite()
    });
    if anchors.is_empty() || price_h <= 0.0 || !price_h.is_finite() {
        return Vec::new();
    }

    if let Some(position_index) = anchors
        .iter()
        .position(|anchor| anchor.kind == RightAxisBadgeKind::PositionEntry)
    {
        return stack_right_axis_badges_around_fixed_position(anchors, position_index, price_h);
    }

    anchors.sort_by(|a, b| {
        a.source_y
            .total_cmp(&b.source_y)
            .then_with(|| a.sort_rank.cmp(&b.sort_rank))
    });

    let min_top = RIGHT_AXIS_BADGE_MARGIN;
    let max_bottom = (price_h - RIGHT_AXIS_BADGE_MARGIN).max(min_top);
    stack_right_axis_badges_in_band(anchors, min_top, max_bottom, true, true)
}

fn stack_right_axis_badges_around_fixed_position(
    mut anchors: Vec<RightAxisBadgeAnchor>,
    position_index: usize,
    price_h: f32,
) -> Vec<StackedRightAxisBadge> {
    let position = anchors.remove(position_index);
    let fixed = StackedRightAxisBadge {
        kind: position.kind,
        source_y: position.source_y,
        badge_y: position.source_y,
        height: position.height,
    };
    let fixed_top = badge_top(fixed);
    let fixed_bottom = badge_bottom(fixed);
    let min_top = RIGHT_AXIS_BADGE_MARGIN;
    let max_bottom = (price_h - RIGHT_AXIS_BADGE_MARGIN).max(min_top);
    let above_max_bottom = fixed_top - RIGHT_AXIS_BADGE_GAP;
    let below_min_top = fixed_bottom + RIGHT_AXIS_BADGE_GAP;
    let mut above = Vec::new();
    let mut below = Vec::new();

    for anchor in anchors {
        match preferred_side_of_fixed_position(anchor, fixed_top, fixed_bottom) {
            FixedBadgeSide::Above => above.push(anchor),
            FixedBadgeSide::Below => below.push(anchor),
        }
    }

    let mut positions =
        stack_right_axis_badges_in_band(above, min_top, above_max_bottom, false, true);
    positions.push(fixed);
    positions.extend(stack_right_axis_badges_in_band(
        below,
        below_min_top,
        max_bottom,
        true,
        false,
    ));
    positions.sort_by(|a, b| {
        a.badge_y
            .total_cmp(&b.badge_y)
            .then_with(|| badge_sort_rank(a.kind).cmp(&badge_sort_rank(b.kind)))
    });
    positions
}

fn preferred_side_of_fixed_position(
    anchor: RightAxisBadgeAnchor,
    fixed_top: f32,
    fixed_bottom: f32,
) -> FixedBadgeSide {
    let anchor_top = anchor.source_y - anchor.height * 0.5;
    let anchor_bottom = anchor.source_y + anchor.height * 0.5;
    if anchor_bottom + RIGHT_AXIS_BADGE_GAP <= fixed_top {
        FixedBadgeSide::Above
    } else if anchor_top - RIGHT_AXIS_BADGE_GAP >= fixed_bottom {
        FixedBadgeSide::Below
    } else if let Some(side) = anchor.fixed_side {
        side
    } else if anchor.source_y <= (fixed_top + fixed_bottom) * 0.5 {
        FixedBadgeSide::Above
    } else {
        FixedBadgeSide::Below
    }
}

fn stack_right_axis_badges_in_band(
    mut anchors: Vec<RightAxisBadgeAnchor>,
    min_top: f32,
    max_bottom: f32,
    protect_top: bool,
    protect_bottom: bool,
) -> Vec<StackedRightAxisBadge> {
    if anchors.is_empty() {
        return Vec::new();
    }

    anchors.sort_by(|a, b| {
        a.source_y
            .total_cmp(&b.source_y)
            .then_with(|| a.sort_rank.cmp(&b.sort_rank))
    });

    let mut positions = Vec::with_capacity(anchors.len());
    let mut next_top = min_top;

    for anchor in anchors {
        let max_top = (max_bottom - anchor.height).max(min_top);
        let desired_top = anchor.source_y - anchor.height * 0.5;
        let top = desired_top.clamp(min_top, max_top).max(next_top);
        positions.push(StackedRightAxisBadge {
            kind: anchor.kind,
            source_y: anchor.source_y,
            badge_y: top + anchor.height * 0.5,
            height: anchor.height,
        });
        next_top = top + anchor.height + RIGHT_AXIS_BADGE_GAP;
    }

    if protect_bottom
        && positions
            .last()
            .is_some_and(|position| badge_bottom(*position) > max_bottom)
    {
        let mut next_bottom = max_bottom;
        for position in positions.iter_mut().rev() {
            let current_top = badge_top(*position);
            let max_top = if protect_top {
                (next_bottom - position.height).max(min_top)
            } else {
                next_bottom - position.height
            };
            let top = current_top.min(max_top);
            position.badge_y = top + position.height * 0.5;
            next_bottom = top - RIGHT_AXIS_BADGE_GAP;
        }

        if protect_top
            && let Some(first) = positions.first()
            && badge_top(*first) < min_top
        {
            let shift = min_top - badge_top(*first);
            for position in &mut positions {
                position.badge_y += shift;
            }
        }
    }

    positions
}

fn badge_sort_rank(kind: RightAxisBadgeKind) -> usize {
    match kind {
        RightAxisBadgeKind::CurrentPrice => RIGHT_AXIS_CURRENT_PRICE_SORT_RANK,
        RightAxisBadgeKind::QuickOrder => RIGHT_AXIS_QUICK_ORDER_SORT_RANK,
        RightAxisBadgeKind::PositionEntry => RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
        RightAxisBadgeKind::PositionLiquidation => RIGHT_AXIS_LIQUIDATION_SORT_RANK,
        RightAxisBadgeKind::ActiveOrder(index) => RIGHT_AXIS_SELL_ORDER_SORT_BASE + index,
        RightAxisBadgeKind::HorizontalAnnotation(index) => RIGHT_AXIS_ANNOTATION_SORT_BASE + index,
    }
}
