use super::rows::{centered_symmetric_side_row_count, side_padding_row_count};
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

// BOOK_ROW_HEIGHT is 20.0, so a side of `h` px fits `floor(h / 20)` rows.

#[test]
fn centered_side_count_clamps_to_the_thinner_side_when_height_allows_more() {
    // 800px fits 40 rows, but asks only have 8 buckets: both sides must show 8
    // so the book is not lopsided (the bug was bids showing 30 while asks
    // showed 8).
    assert_eq!(centered_symmetric_side_row_count(800.0, 8, 30), 8);
    assert_eq!(centered_symmetric_side_row_count(800.0, 30, 8), 8);
}

#[test]
fn centered_side_count_is_independent_of_argument_order() {
    // Whatever the per-side bucket counts, both columns derive the same row
    // count, so swapping which side is thinner never changes the result.
    for (asks, bids) in [(8usize, 30usize), (0, 40), (15, 15), (40, 2)] {
        assert_eq!(
            centered_symmetric_side_row_count(800.0, asks, bids),
            centered_symmetric_side_row_count(800.0, bids, asks),
        );
    }
}

#[test]
fn centered_side_count_uses_height_when_both_sides_are_deep() {
    // Equal, plentiful depth: the height is the only limiter and both sides
    // agree.
    assert_eq!(centered_symmetric_side_row_count(400.0, 40, 40), 20);
    assert_eq!(centered_symmetric_side_row_count(100.0, 8, 30), 5);
}

#[test]
fn centered_side_count_is_zero_for_empty_side_or_no_height() {
    assert_eq!(centered_symmetric_side_row_count(800.0, 0, 30), 0);
    assert_eq!(centered_symmetric_side_row_count(0.0, 10, 10), 0);
    assert_eq!(centered_symmetric_side_row_count(-5.0, 10, 10), 0);
}

#[test]
fn centered_side_count_floors_a_sub_row_height_to_zero() {
    // A positive height shorter than one row (BOOK_ROW_HEIGHT = 20.0) floors
    // to zero rows on both sides rather than rendering a clipped half-row.
    assert_eq!(centered_symmetric_side_row_count(15.0, 10, 10), 0);
}
