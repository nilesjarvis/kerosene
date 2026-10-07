use super::rows::{centered_order_book_side_row_count, side_padding_row_count};
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

#[test]
fn side_padding_fills_each_side_up_to_the_fixed_row_count() {
    assert_eq!(side_padding_row_count(12, 40), 28);
    assert_eq!(side_padding_row_count(40, 40), 0);
    assert_eq!(side_padding_row_count(45, 40), 0);
    assert_eq!(side_padding_row_count(0, 40), 40);
}

// BOOK_ROW_HEIGHT is 20.0. Each side uses its own available depth; an empty
// or thinner side must never hide real levels on the other side.
#[test]
fn centered_side_count_keeps_available_depth_within_the_viewport() {
    assert_eq!(centered_order_book_side_row_count(800.0, 8), 8);
    assert_eq!(centered_order_book_side_row_count(800.0, 20), 20);
    assert_eq!(centered_order_book_side_row_count(100.0, 20), 5);
    assert_eq!(centered_order_book_side_row_count(800.0, 0), 0);
    assert_eq!(centered_order_book_side_row_count(0.0, 20), 0);
    assert_eq!(centered_order_book_side_row_count(-5.0, 20), 0);
    assert_eq!(centered_order_book_side_row_count(15.0, 20), 0);
}
