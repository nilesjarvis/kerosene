use super::fill;
use crate::journal::{merge_fills, newest_fill_time, normalize_fills};

#[test]
fn normalize_fills_sorts_and_deduplicates_by_composite_identity() {
    let duplicate = fill(3, 30, "ETH");
    let mut fills = vec![
        duplicate.clone(),
        fill(1, 10, "BTC"),
        duplicate,
        fill(2, 20, "SOL"),
    ];

    normalize_fills(&mut fills);

    assert_eq!(fills.len(), 3);
    assert_eq!(fills[0].time, 1);
    assert_eq!(fills[1].time, 2);
    assert_eq!(fills[2].time, 3);
}

#[test]
fn merge_fills_uses_composite_identity_not_tid_only() {
    let mut existing = vec![fill(1, 10, "BTC")];
    let mut same_tid_different_fill = fill(2, 10, "ETH");
    same_tid_different_fill.hash = "0xdifferent".to_string();

    let added = merge_fills(
        &mut existing,
        vec![fill(1, 10, "BTC"), same_tid_different_fill],
    );

    assert_eq!(added, 1);
    assert_eq!(existing.len(), 2);
    assert_eq!(newest_fill_time(&existing), Some(2));
}

#[test]
fn merge_fills_deduplicates_inclusive_page_boundaries() {
    let mut existing = vec![fill(1, 10, "BTC"), fill(2, 20, "BTC")];

    let added = merge_fills(
        &mut existing,
        vec![fill(2, 20, "BTC"), fill(3, 30, "BTC"), fill(4, 40, "BTC")],
    );

    assert_eq!(added, 2);
    assert_eq!(existing.len(), 4);
    assert_eq!(
        existing.iter().map(|fill| fill.time).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}
