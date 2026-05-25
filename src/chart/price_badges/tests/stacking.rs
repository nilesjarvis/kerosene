use super::*;

#[test]
fn right_axis_badges_stack_nearby_labels() {
    let positions = stack_right_axis_badge_positions(
        vec![
            anchor(RightAxisBadgeKind::CurrentPrice, 40.0, 16.0, 0, None),
            anchor(RightAxisBadgeKind::QuickOrder, 42.0, 16.0, 1, None),
            anchor(RightAxisBadgeKind::ActiveOrder(0), 43.0, 14.0, 2, None),
        ],
        120.0,
    );

    assert_eq!(positions.len(), 3);
    assert_non_overlapping(&positions);
}

#[test]
fn right_axis_badges_pack_back_inside_bottom_edge() {
    let positions = stack_right_axis_badge_positions(
        vec![
            anchor(RightAxisBadgeKind::CurrentPrice, 96.0, 16.0, 0, None),
            anchor(RightAxisBadgeKind::QuickOrder, 98.0, 16.0, 1, None),
        ],
        100.0,
    );

    assert_eq!(positions.len(), 2);
    assert_non_overlapping(&positions);
    assert!(badge_bottom(positions[1]) <= 98.0);
}
