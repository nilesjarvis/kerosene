use super::{max_cumulative_depth, max_level_size};

#[test]
fn max_cumulative_depth_uses_largest_value_after_ask_rows_are_reversed() {
    let ask_rows = vec![(101.0, 3.0, 6.0), (100.5, 2.0, 3.0), (100.0, 1.0, 1.0)];

    assert_eq!(max_cumulative_depth(&ask_rows), 6.0);
}

#[test]
fn max_cumulative_depth_never_drops_below_one_for_empty_or_tiny_books() {
    assert_eq!(max_cumulative_depth(&[]), 1.0);
    assert_eq!(max_cumulative_depth(&[(100.0, 0.25, 0.25)]), 1.0);
}

#[test]
fn max_level_size_uses_both_sides() {
    let asks = vec![(101.0, 2.0, 2.0)];
    let bids = vec![(99.0, 5.0, 5.0)];

    assert_eq!(max_level_size(&asks, &bids), 5.0);
}
