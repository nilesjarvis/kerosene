use super::fill;
use crate::api::UserFill;
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

fn assert_same_fills(actual: &[UserFill], expected: &[UserFill]) {
    // These fixtures are synthetic; compare every payload field, not just IDs.
    assert_eq!(
        serde_json::to_value(actual).expect("serialize synthetic fills"),
        serde_json::to_value(expected).expect("serialize expected synthetic fills")
    );
}

#[test]
fn normalization_uses_every_identity_field_and_retains_the_first_complete_payload() {
    let mut first = fill(10, 20, "TEST-A");
    first.start_position = "invalid".to_string();
    let mut expected = vec![first.clone()];
    let changes: [fn(&mut UserFill); 8] = [
        |fill| fill.sz.push('0'),
        |fill| fill.px.push('0'),
        |fill| fill.side.push('Z'),
        |fill| fill.hash.push('Z'),
        |fill| fill.oid += 1,
        |fill| fill.tid += 1,
        |fill| fill.coin.push('Z'),
        |fill| fill.time += 1,
    ];
    for change in changes {
        let mut distinct = first.clone();
        change(&mut distinct);
        expected.push(distinct);
    }

    let mut later_duplicate = first.clone();
    later_duplicate.start_position = "3".to_string();
    later_duplicate.dir = "Close Long".to_string();
    later_duplicate.closed_pnl = "42".to_string();
    later_duplicate.crossed = true;
    later_duplicate.fee = "0.75".to_string();
    later_duplicate.fee_token = "TEST-TOKEN".to_string();

    let mut input = expected.iter().rev().cloned().collect::<Vec<_>>();
    input.insert(0, first);
    input.push(later_duplicate);
    normalize_fills(&mut input);
    assert_same_fills(&input, &expected);
    normalize_fills(&mut input);
    assert_same_fills(&input, &expected);
}

#[test]
fn normalization_preserves_complete_payloads_across_position_chain_groups() {
    // Expected execution order, including disconnected chains, an invalid group,
    // a cycle with no head, and a settlement that leaves position size unchanged.
    let expected = [
        (1, 99, "TEST-A", "7", "B", "Open Long"),
        (2, 30, "TEST-A", "0", "B", "Open Long"),
        (2, 10, "TEST-A", "1", "B", "Open Long"),
        (2, 20, "TEST-A", "2", "B", "Open Long"),
        (2, 20, "TEST-B", "0", "B", "Open Long"),
        (2, 10, "TEST-B", "1", "B", "Open Long"),
        (2, 30, "TEST-C", "0", "B", "Open Long"),
        (2, 20, "TEST-C", "1", "B", "Open Long"),
        (2, 40, "TEST-C", "4", "B", "Open Long"),
        (2, 10, "TEST-C", "5", "B", "Open Long"),
        (2, 10, "TEST-D", "1", "A", "Close Long"),
        (2, 20, "TEST-D", "0", "B", "Open Long"),
        (3, 20, "TEST-A", "0", "B", "Open Long"),
        (3, 10, "TEST-A", "1", "B", "Settlement"),
        (4, 10, "TEST-A", "1", "B", "Open Long"),
        (4, 20, "TEST-A", "NaN", "B", "Open Long"),
        (4, 30, "TEST-A", "0", "B", "Open Long"),
        (5, 99, "TEST-A", "7", "B", "Open Long"),
    ]
    .into_iter()
    .map(|(time, tid, coin, start, side, dir)| {
        let mut fill = fill(time, tid, coin);
        fill.start_position = start.to_string();
        fill.side = side.to_string();
        fill.dir = dir.to_string();
        fill.fee = format!("0.{tid}");
        fill.closed_pnl = format!("{time}.{tid}");
        fill
    })
    .collect::<Vec<_>>();

    for expected in [Vec::new(), vec![fill(1, 1, "TEST")], expected] {
        let mut input = expected.iter().rev().cloned().collect::<Vec<_>>();
        normalize_fills(&mut input);
        assert_same_fills(&input, &expected);
        normalize_fills(&mut input);
        assert_same_fills(&input, &expected);
    }
}

#[test]
fn merging_counts_new_identities_and_preserves_first_existing_and_incoming_payloads() {
    let first = fill(1, 10, "TEST");
    let second = fill(2, 20, "TEST");
    let mut old_duplicate = first.clone();
    old_duplicate.fee = "99".to_string();
    let mut new_duplicate = second.clone();
    new_duplicate.fee = "88".to_string();
    let mut existing = vec![first.clone(), old_duplicate.clone()];

    let added = merge_fills(
        &mut existing,
        vec![old_duplicate, second.clone(), new_duplicate],
    );
    assert_eq!(added, 1);
    assert_same_fills(&existing, &[first, second]);
    assert_eq!(merge_fills(&mut existing, Vec::new()), 0);
}
