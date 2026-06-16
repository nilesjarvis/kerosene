use super::layout::*;

mod annotations;
mod fixed;
mod stacking;

fn anchor(
    kind: RightAxisBadgeKind,
    source_y: f32,
    height: f32,
    sort_rank: usize,
    fixed_side: Option<FixedBadgeSide>,
) -> RightAxisBadgeAnchor {
    RightAxisBadgeAnchor {
        kind,
        source_y,
        height,
        sort_rank,
        fixed_side,
    }
}

fn assert_non_overlapping(positions: &[StackedRightAxisBadge]) {
    for pair in positions.windows(2) {
        assert!(
            badge_bottom(pair[0]) + RIGHT_AXIS_BADGE_GAP <= badge_top(pair[1]),
            "{:?} overlaps {:?}",
            pair[0],
            pair[1]
        );
    }
}

fn badge_for_or_panic(
    positions: &[StackedRightAxisBadge],
    kind: RightAxisBadgeKind,
) -> StackedRightAxisBadge {
    for position in positions {
        if position.kind == kind {
            return *position;
        }
    }

    panic!("missing {kind:?} badge");
}
