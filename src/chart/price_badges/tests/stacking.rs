use super::RightAxisBadgeKind::{ActiveOrder, CurrentPrice, HorizontalAnnotation, QuickOrder};
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

#[test]
fn right_axis_badge_packing_preserves_stable_ties_and_crowded_edges() {
    let anchors = vec![
        anchor(ActiveOrder(2), 40.0, 14.0, 5, None),
        anchor(QuickOrder, 40.0, 16.0, 5, None),
        anchor(CurrentPrice, -0.0, 16.0, 100, None),
        anchor(ActiveOrder(0), 0.0, 14.0, 0, None),
        anchor(HorizontalAnnotation(1), 40.0, 14.0, 3, None),
        anchor(ActiveOrder(9), f32::NAN, 14.0, 0, None),
        anchor(ActiveOrder(8), 70.0, 0.0, 0, None),
        anchor(ActiveOrder(7), 90.0, f32::INFINITY, 0, None),
    ];
    let kinds = [
        CurrentPrice,
        ActiveOrder(0),
        HorizontalAnnotation(1),
        ActiveOrder(2),
        QuickOrder,
    ];
    for (height, expected) in [
        (200.0, [10.0, 27.0, 43.0, 59.0, 76.0]),
        (60.0, [10.0, 9.0, 17.0, 33.0, 50.0]),
    ] {
        let positions = stack_right_axis_badge_positions(anchors.clone(), height);
        assert_eq!(positions.len(), kinds.len());
        for (index, position) in positions.iter().enumerate() {
            assert_eq!(position.kind, kinds[index]);
            assert_eq!(position.badge_y, expected[index]);
            assert_eq!(position.height, [16.0, 14.0, 14.0, 14.0, 16.0][index]);
            assert_eq!(
                position.source_y.to_bits(),
                [-0.0_f32, 0.0, 40.0, 40.0, 40.0][index].to_bits()
            );
        }
    }
    for height in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(stack_right_axis_badge_positions(anchors.clone(), height).is_empty());
    }
}
