use super::*;

#[test]
fn right_axis_order_tie_keeps_sells_above_buys() {
    let positions = stack_right_axis_badge_positions(
        vec![
            anchor(
                RightAxisBadgeKind::ActiveOrder(0),
                50.0,
                14.0,
                RIGHT_AXIS_BUY_ORDER_SORT_BASE,
                Some(FixedBadgeSide::Below),
            ),
            anchor(
                RightAxisBadgeKind::PositionEntry,
                50.0,
                16.0,
                RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
                None,
            ),
            anchor(
                RightAxisBadgeKind::ActiveOrder(1),
                50.0,
                14.0,
                RIGHT_AXIS_SELL_ORDER_SORT_BASE,
                Some(FixedBadgeSide::Above),
            ),
        ],
        160.0,
    );

    assert_eq!(positions[0].kind, RightAxisBadgeKind::ActiveOrder(1));
    assert_eq!(positions[1].kind, RightAxisBadgeKind::PositionEntry);
    assert_eq!(positions[2].kind, RightAxisBadgeKind::ActiveOrder(0));
    assert_eq!(positions[1].badge_y, 50.0);
    assert_non_overlapping(&positions);
}

#[test]
fn right_axis_position_entry_stays_fixed_when_limit_orders_overlap() {
    let positions = stack_right_axis_badge_positions(
        vec![
            anchor(
                RightAxisBadgeKind::PositionEntry,
                50.0,
                16.0,
                RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
                None,
            ),
            anchor(
                RightAxisBadgeKind::ActiveOrder(0),
                50.0,
                14.0,
                RIGHT_AXIS_SELL_ORDER_SORT_BASE,
                Some(FixedBadgeSide::Above),
            ),
            anchor(
                RightAxisBadgeKind::ActiveOrder(1),
                50.0,
                14.0,
                RIGHT_AXIS_BUY_ORDER_SORT_BASE,
                Some(FixedBadgeSide::Below),
            ),
        ],
        120.0,
    );
    let position = badge_for_or_panic(&positions, RightAxisBadgeKind::PositionEntry);
    let sell = badge_for_or_panic(&positions, RightAxisBadgeKind::ActiveOrder(0));
    let buy = badge_for_or_panic(&positions, RightAxisBadgeKind::ActiveOrder(1));

    assert_eq!(position.badge_y, 50.0);
    assert!(badge_bottom(sell) + RIGHT_AXIS_BADGE_GAP <= badge_top(position));
    assert!(badge_top(buy) - RIGHT_AXIS_BADGE_GAP >= badge_bottom(position));
}
