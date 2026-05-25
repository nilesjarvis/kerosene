use super::super::RightAxisBadgeKind;

// ---------------------------------------------------------------------------
// Badge Stacking Model
// ---------------------------------------------------------------------------

pub(in crate::chart::price_badges) const RIGHT_AXIS_BADGE_GAP: f32 = 2.0;
pub(super) const RIGHT_AXIS_BADGE_MARGIN: f32 = 2.0;
pub(in crate::chart::price_badges) const RIGHT_AXIS_SELL_ORDER_SORT_BASE: usize = 20_000;
pub(in crate::chart::price_badges) const RIGHT_AXIS_CURRENT_PRICE_SORT_RANK: usize = 40_000;
pub(in crate::chart::price_badges) const RIGHT_AXIS_POSITION_ENTRY_SORT_RANK: usize = 50_000;
pub(in crate::chart::price_badges) const RIGHT_AXIS_QUICK_ORDER_SORT_RANK: usize = 60_000;
pub(in crate::chart::price_badges) const RIGHT_AXIS_LIQUIDATION_SORT_RANK: usize = 70_000;
pub(in crate::chart::price_badges) const RIGHT_AXIS_BUY_ORDER_SORT_BASE: usize = 80_000;
pub(in crate::chart::price_badges) const RIGHT_AXIS_ANNOTATION_SORT_BASE: usize = 90_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::chart::price_badges) enum FixedBadgeSide {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart::price_badges) struct RightAxisBadgeAnchor {
    pub(in crate::chart::price_badges) kind: RightAxisBadgeKind,
    pub(in crate::chart::price_badges) source_y: f32,
    pub(in crate::chart::price_badges) height: f32,
    pub(in crate::chart::price_badges) sort_rank: usize,
    pub(in crate::chart::price_badges) fixed_side: Option<FixedBadgeSide>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart::price_badges) struct StackedRightAxisBadge {
    pub(in crate::chart::price_badges) kind: RightAxisBadgeKind,
    pub(in crate::chart::price_badges) source_y: f32,
    pub(in crate::chart::price_badges) badge_y: f32,
    pub(in crate::chart::price_badges) height: f32,
}

pub(in crate::chart::price_badges) fn push_visible_badge(
    anchors: &mut Vec<RightAxisBadgeAnchor>,
    kind: RightAxisBadgeKind,
    source_y: f32,
    height: f32,
    sort_rank: usize,
    fixed_side: Option<FixedBadgeSide>,
    price_h: f32,
) {
    if source_y >= -10.0
        && source_y <= price_h + 10.0
        && source_y.is_finite()
        && height > 0.0
        && height.is_finite()
    {
        anchors.push(RightAxisBadgeAnchor {
            kind,
            source_y,
            height,
            sort_rank,
            fixed_side,
        });
    }
}

pub(in crate::chart::price_badges) fn badge_top(position: StackedRightAxisBadge) -> f32 {
    position.badge_y - position.height * 0.5
}

pub(in crate::chart::price_badges) fn badge_bottom(position: StackedRightAxisBadge) -> f32 {
    position.badge_y + position.height * 0.5
}
