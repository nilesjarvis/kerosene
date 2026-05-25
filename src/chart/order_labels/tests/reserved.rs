use super::*;

#[test]
fn order_label_stack_avoids_reserved_position_label() {
    let positions = stack_order_label_positions_avoiding(
        vec![anchor(0, 40.0, true)],
        240.0,
        &[position_label_range(40.0)],
    );

    assert_eq!(positions[0].label_y, 57.0);
}

#[test]
fn order_label_stack_keeps_asks_above_position_label() {
    let positions = stack_order_label_positions_avoiding(
        vec![anchor(0, 40.0, false)],
        240.0,
        &[position_label_range(40.0)],
    );

    assert_eq!(positions[0].label_y, 23.0);
}

#[test]
fn order_label_stack_keeps_bid_labels_below_position_label() {
    let positions = stack_order_label_positions_avoiding(
        vec![anchor(0, 40.0, true), anchor(1, 41.0, true)],
        240.0,
        &[position_label_range(40.0)],
    );

    assert_eq!(positions[0].label_y, 57.0);
    assert_eq!(positions[1].label_y, 71.0);
}
