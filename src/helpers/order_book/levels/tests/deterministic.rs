use super::*;

#[test]
fn nice_step_ceil_quantizes_to_1_2_5_ladder() {
    assert_eq!(nice_step_ceil(0.0), 1.0);
    assert_eq!(nice_step_ceil(1.0), 1.0);
    assert_eq!(nice_step_ceil(1.4), 2.0);
    assert_eq!(nice_step_ceil(2.0), 2.0);
    assert_eq!(nice_step_ceil(3.7), 5.0);
    assert_eq!(nice_step_ceil(7.2), 10.0);
    assert_eq!(nice_step_ceil(130.0), 200.0);
    assert_eq!(nice_step_ceil(4_900.0), 5_000.0);
    assert_eq!(nice_step_ceil(f64::NAN), 1.0);
}

#[test]
fn nice_step_ceil_is_stable_within_a_band() {
    // Per-frame wiggles inside one band keep the same normalizer — the
    // reason the depth bars stop rescaling on every update.
    assert_eq!(nice_step_ceil(3.1), nice_step_ceil(4.9));
    assert_eq!(nice_step_ceil(1_050.0), nice_step_ceil(1_900.0));
}

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
