use super::{SNAPSHOT_SCOPE, active_instance, apply_update, bid_prices};

#[test]
fn same_precision_empty_side_replaces_snapshot() {
    let mut inst = active_instance(
        &[(100.0, 10.0), (95.0, 20.0)],
        &[(105.0, 10.0), (110.0, 20.0)],
    );

    apply_update(&mut inst, &[(99.0, 1.0)], &[], SNAPSHOT_SCOPE);

    assert_eq!(bid_prices(&inst), vec![99.0]);
    assert!(inst.book.asks.is_empty());
}

#[test]
fn same_precision_update_replaces_snapshot() {
    let mut inst = active_instance(
        &[(100.0, 10.0), (95.0, 20.0)],
        &[(105.0, 10.0), (110.0, 20.0)],
    );

    apply_update(&mut inst, &[(100.0, 1.0)], &[(105.0, 1.0)], SNAPSHOT_SCOPE);

    assert_eq!(inst.book.bids.len(), 1);
    assert_eq!(inst.book.asks.len(), 1);
    assert_eq!(inst.book.bids[0].sz, 1.0);
    assert_eq!(inst.book.asks[0].sz, 1.0);
}
