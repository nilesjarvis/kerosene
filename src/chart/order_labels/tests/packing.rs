use super::*;

#[test]
fn order_label_packing_preserves_reserved_bands_and_crowded_edge_positions() {
    let anchors = vec![
        anchor(3, 96.0, false),
        anchor(2, 40.0, false),
        anchor(1, 40.0, true),
        anchor(0, 12.0, true),
    ];
    let cases = [
        (100.0, vec![], [12.0, 40.0, 54.0, 92.0]),
        (100.0, vec![40.0], [9.0, 57.0, 23.0, 92.0]),
        (100.0, vec![40.0, 80.0], [8.0, 22.0, 36.0, 76.0]),
        (100.0, vec![80.0, 40.0], [8.0, 22.0, 36.0, 76.0]),
        (240.0, vec![40.0, 80.0], [12.0, 57.0, 97.0, 111.0]),
        (12.0, vec![40.0], [9.0, 8.0, 23.0, 22.0]),
        (12.0, vec![], [8.0, 22.0, 36.0, 50.0]),
        (0.0, vec![], [8.0, 22.0, 36.0, 50.0]),
        (-5.0, vec![], [8.0, 22.0, 36.0, 50.0]),
        (f32::NAN, vec![], [8.0, 22.0, 36.0, 50.0]),
        (f32::INFINITY, vec![], [12.0, 40.0, 54.0, 96.0]),
    ];
    for (case, (height, centers, expected)) in cases.into_iter().enumerate() {
        let reserved: Vec<_> = centers.into_iter().map(position_label_range).collect();
        let positions = stack_order_label_positions_avoiding(anchors.clone(), height, &reserved);
        assert_eq!(positions.len(), expected.len(), "case {case}");
        for (index, position) in positions.iter().enumerate() {
            assert_eq!(position.order_index, index, "case {case}");
            assert_eq!(
                position.order_y,
                [12.0, 40.0, 40.0, 96.0][index],
                "case {case}"
            );
            assert_eq!(position.label_y, expected[index], "case {case}");
        }
        assert!(stack_order_label_positions_avoiding(Vec::new(), height, &reserved).is_empty());
    }
}

#[test]
fn order_label_packing_preserves_duplicate_keys_across_reserved_bands() {
    for reserved in [vec![], vec![position_label_range(40.0)]] {
        let positions = stack_order_label_positions_avoiding(
            vec![
                anchor(2, 40.0, true),
                anchor(1, 40.0, true),
                anchor(1, 40.0, false),
            ],
            240.0,
            &reserved,
        );
        let labels: Vec<_> = positions
            .iter()
            .map(|position| (position.order_index, position.label_y))
            .collect();
        let expected = if reserved.is_empty() {
            vec![(1, 40.0), (1, 54.0), (2, 68.0)]
        } else {
            // Reserved bands collect asks before bids, then stably sort by index.
            vec![(1, 23.0), (1, 57.0), (2, 71.0)]
        };
        assert_eq!(labels, expected);
    }
}
