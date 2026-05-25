use super::*;

#[test]
fn order_label_stack_separates_nearby_orders() {
    let positions = stack_order_label_positions_avoiding(
        vec![
            anchor(0, 40.0, true),
            anchor(1, 42.0, true),
            anchor(2, 43.0, true),
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
        vec![anchor(0, 96.0, true), anchor(1, 98.0, true)],
        100.0,
        &[],
    );

    assert_eq!(positions[0].label_y, 78.0);
    assert_eq!(positions[1].label_y, 92.0);
}
