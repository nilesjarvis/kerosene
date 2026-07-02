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

#[test]
fn grid_aligned_bids_stay_on_their_own_tick_row() {
    // 63.239 / 0.001 = 63238.99999999999 in f64: an unguarded floor shifted
    // this bid onto the 63.238 row and merged its size with the real 63.238
    // level.
    let bids = [lvl(63.239, 1.0), lvl(63.238, 2.0)];
    let rows = aggregate_levels(&bids, 0.001, true);
    assert_eq!(rows.len(), 2);
    assert!((rows[0].0 - 63.239).abs() < 1e-9, "top row {}", rows[0].0);
    assert_eq!(rows[0].1, 1.0);
    assert!((rows[1].0 - 63.238).abs() < 1e-9, "next row {}", rows[1].0);
    assert_eq!(rows[1].1, 2.0);
}

#[test]
fn grid_aligned_asks_stay_on_their_own_tick_row() {
    // 8.002 / 0.002 = 4001.0000000000005 in f64: an unguarded ceil shifted
    // this ask onto the 8.004 row.
    let rows = aggregate_levels(&[lvl(8.002, 1.5)], 0.002, false);
    assert_eq!(rows.len(), 1);
    assert!((rows[0].0 - 8.002).abs() < 1e-9, "ask row {}", rows[0].0);
    assert_eq!(rows[0].1, 1.5);
}

#[test]
fn grid_aligned_prices_at_8_decimal_spot_ticks_keep_their_rows() {
    // Spot pairs quote up to 8 decimal places. Both float-division failure
    // modes occur at tick 1e-8: 0.00001013 / 1e-8 = 1012.9999999999999
    // (bid floor drops a tick) and 0.00001018 / 1e-8 = 1018.0000000000001
    // (ask ceil gains a tick).
    let tick = 1e-8;
    let bids = aggregate_levels(&[lvl(0.00001013, 3.0)], tick, true);
    assert_eq!(bids.len(), 1);
    assert!(
        (bids[0].0 - 0.00001013).abs() < 1e-13,
        "bid row {}",
        bids[0].0
    );
    assert_eq!(bids[0].1, 3.0);
    let asks = aggregate_levels(&[lvl(0.00001018, 4.0)], tick, false);
    assert_eq!(asks.len(), 1);
    assert!(
        (asks[0].0 - 0.00001018).abs() < 1e-13,
        "ask row {}",
        asks[0].0
    );
    assert_eq!(asks[0].1, 4.0);
}

#[test]
fn off_grid_prices_still_round_toward_the_book_inside() {
    // A price halfway between ticks is far outside the snap tolerance and
    // must keep the directional semantics: bids bucket down, asks bucket up.
    let bids = aggregate_levels(&[lvl(63.2385, 1.0)], 0.001, true);
    assert!((bids[0].0 - 63.238).abs() < 1e-9, "bid row {}", bids[0].0);
    let asks = aggregate_levels(&[lvl(63.2385, 1.0)], 0.001, false);
    assert!((asks[0].0 - 63.239).abs() < 1e-9, "ask row {}", asks[0].0);
}
