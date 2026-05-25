use super::*;

#[test]
fn empty_levels_aggregate_to_empty_buckets() {
    assert!(aggregate_levels(&[], 0.5, true).is_empty());
    assert!(aggregate_levels(&[], 0.5, false).is_empty());
}

#[test]
fn single_level_emits_one_bucket() {
    let asks = [lvl(100.5, 2.0)];
    let bids = [lvl(99.5, 2.0)];
    assert_eq!(aggregate_levels(&asks, 0.5, false), vec![(100.5, 2.0)]);
    assert_eq!(aggregate_levels(&bids, 0.5, true), vec![(99.5, 2.0)]);
}
