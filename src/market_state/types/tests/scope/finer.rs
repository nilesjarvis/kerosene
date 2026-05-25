use super::{active_instance, apply_update, ask_prices, bid_prices};

const FINER_SCOPE: f64 = 1.0;

#[test]
fn finer_live_update_preserves_coarse_snapshot_scope() {
    let mut inst = active_instance(
        &[(100.0, 10.0), (95.0, 20.0), (90.0, 30.0)],
        &[(105.0, 10.0), (110.0, 20.0), (115.0, 30.0)],
    );

    apply_update(
        &mut inst,
        &[(100.0, 1.0), (99.0, 2.0)],
        &[(101.0, 1.0), (102.0, 2.0)],
        FINER_SCOPE,
    );

    assert_eq!(bid_prices(&inst), vec![100.0, 99.0, 95.0, 90.0]);
    assert_eq!(ask_prices(&inst), vec![101.0, 102.0, 105.0, 110.0, 115.0]);
    assert_eq!(inst.book.bids[0].sz, 1.0);
    assert_eq!(inst.book.asks[0].sz, 1.0);
}

#[test]
fn finer_bid_update_drops_stale_bids_above_fresh_scope() {
    let mut inst = active_instance(
        &[(105.0, 10.0), (100.0, 20.0), (95.0, 30.0)],
        &[(110.0, 10.0), (115.0, 20.0)],
    );

    apply_update(
        &mut inst,
        &[(100.0, 1.0), (99.0, 2.0)],
        &[(110.0, 1.0), (111.0, 2.0)],
        FINER_SCOPE,
    );

    assert_eq!(bid_prices(&inst), vec![100.0, 99.0, 95.0]);
    assert_eq!(inst.book.bids[0].sz, 1.0);
    assert!(!inst.book.bids.iter().any(|level| level.px == 105.0));
}

#[test]
fn finer_ask_update_drops_stale_asks_below_fresh_scope() {
    let mut inst = active_instance(
        &[(95.0, 10.0), (90.0, 20.0)],
        &[(101.0, 10.0), (105.0, 20.0), (115.0, 30.0)],
    );

    apply_update(
        &mut inst,
        &[(95.0, 1.0), (94.0, 2.0)],
        &[(110.0, 1.0), (111.0, 2.0)],
        FINER_SCOPE,
    );

    assert_eq!(ask_prices(&inst), vec![110.0, 111.0, 115.0]);
    assert_eq!(inst.book.asks[0].sz, 1.0);
    assert!(!inst.book.asks.iter().any(|level| level.px == 101.0));
    assert!(!inst.book.asks.iter().any(|level| level.px == 105.0));
}

#[test]
fn one_sided_finer_update_applies_non_empty_side_and_clears_empty_side() {
    let mut inst = active_instance(
        &[(100.0, 10.0), (95.0, 20.0)],
        &[(105.0, 10.0), (110.0, 20.0)],
    );

    apply_update(&mut inst, &[(99.0, 1.0)], &[], FINER_SCOPE);

    assert_eq!(bid_prices(&inst), vec![99.0, 95.0]);
    assert_eq!(inst.book.bids[0].sz, 1.0);
    assert!(inst.book.asks.is_empty());
}

#[test]
fn empty_finer_update_clears_existing_book() {
    let mut inst = active_instance(
        &[(100.0, 10.0), (95.0, 20.0)],
        &[(105.0, 10.0), (110.0, 20.0)],
    );

    apply_update(&mut inst, &[], &[], FINER_SCOPE);

    assert!(inst.book.bids.is_empty());
    assert!(inst.book.asks.is_empty());
}

#[test]
fn cleared_side_repopulates_from_later_finer_update() {
    let mut inst = active_instance(
        &[(100.0, 10.0), (95.0, 20.0)],
        &[(105.0, 10.0), (110.0, 20.0)],
    );

    apply_update(&mut inst, &[(99.0, 1.0)], &[], FINER_SCOPE);
    assert!(inst.book.asks.is_empty());

    apply_update(&mut inst, &[(98.0, 2.0)], &[(101.0, 3.0)], FINER_SCOPE);

    assert_eq!(bid_prices(&inst), vec![98.0, 95.0]);
    assert_eq!(ask_prices(&inst), vec![101.0]);
    assert_eq!(inst.book.asks[0].sz, 3.0);
}
