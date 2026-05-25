use super::super::order_book::aggregate_with_cumulative;
use super::lvl;

#[test]
fn aggregate_with_cumulative_returns_empty_for_no_levels() {
    let out = aggregate_with_cumulative(&[], 0.5, false);
    assert!(out.is_empty());
}

#[test]
fn aggregate_with_cumulative_accumulates_asks_from_inside_out() {
    let asks = [lvl(100.0, 1.0), lvl(100.5, 2.0), lvl(101.0, 3.0)];
    let out = aggregate_with_cumulative(&asks, 0.5, false);

    assert_eq!(out.len(), 3);
    assert_eq!(out[0], (100.0, 1.0, 1.0));
    assert_eq!(out[1], (100.5, 2.0, 3.0));
    assert_eq!(out[2], (101.0, 3.0, 6.0));
}

#[test]
fn aggregate_with_cumulative_accumulates_bids_from_inside_out() {
    let bids = [lvl(99.0, 1.5), lvl(98.5, 2.0), lvl(98.0, 0.5)];
    let out = aggregate_with_cumulative(&bids, 0.5, true);

    assert_eq!(out.len(), 3);
    assert_eq!(out[0], (99.0, 1.5, 1.5));
    assert_eq!(out[1], (98.5, 2.0, 3.5));
    assert_eq!(out[2], (98.0, 0.5, 4.0));
}

#[test]
fn aggregate_with_cumulative_groups_sub_tick_levels_into_buckets() {
    // Three sub-tick asks at 99.7 / 99.8 / 99.9 all ceil into the 100.0
    // bucket at tick=0.5; the merged size + cumulative reflect that.
    let asks = [lvl(99.7, 1.0), lvl(99.8, 2.0), lvl(99.9, 4.0)];
    let out = aggregate_with_cumulative(&asks, 0.5, false);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, 100.0);
    assert_eq!(out[0].1, 7.0);
    assert_eq!(out[0].2, 7.0);
}
