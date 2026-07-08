use super::super::{
    apply_fills_update, chase_fill_summary, fill_toast_message, prepend_recent_fills,
};
use super::fixtures::{fill, fill_with_oid};

#[test]
fn recent_fills_are_prepended_without_reversing_incoming_order() {
    let mut existing = vec![fill(3), fill(4)];

    prepend_recent_fills(&mut existing, vec![fill(1), fill(2)], 10);

    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![1, 2, 3, 4]);
}

#[test]
fn recent_fills_are_truncated_before_old_history() {
    let mut existing = vec![fill(3), fill(4), fill(5)];

    prepend_recent_fills(&mut existing, vec![fill(1), fill(2)], 4);

    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![1, 2, 3, 4]);
}

#[test]
fn recent_fills_drop_extra_incoming_when_batch_exceeds_limit() {
    let mut existing = vec![fill(10)];

    prepend_recent_fills(&mut existing, vec![fill(1), fill(2), fill(3)], 2);

    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![1, 2]);
}

#[test]
fn live_fill_updates_preserve_rest_seeded_fill_depth() {
    // The REST bootstrap seeds up to 2000 fills, and spot cost-basis
    // estimation replays deep acquisition fills from that history. A live
    // fill merge must keep it instead of truncating to a short recent window.
    let mut existing: Vec<_> = (2..=300).map(fill).collect();

    let toast_fills = apply_fills_update(&mut existing, vec![fill(1)], false, |_| false);

    assert_eq!(toast_fills.len(), 1);
    assert_eq!(existing.len(), 300);
    assert_eq!(existing.first().map(|fill| fill.time), Some(1));
    assert_eq!(existing.last().map(|fill| fill.time), Some(300));
}

#[test]
fn live_fill_updates_cap_history_at_rest_depth() {
    let mut existing: Vec<_> = (2..=2001).map(fill).collect();

    let _ = apply_fills_update(&mut existing, vec![fill(1)], false, |_| false);

    assert_eq!(existing.len(), 2000);
    assert_eq!(existing.first().map(|fill| fill.time), Some(1));
    assert_eq!(existing.last().map(|fill| fill.time), Some(2000));
}

#[test]
fn fill_snapshot_replaces_existing_history_without_dropping_hidden_symbols() {
    let mut existing = vec![fill(10)];
    let mut hidden_fill = fill(1);
    hidden_fill.coin = "ETH".to_string();

    let toasts = apply_fills_update(&mut existing, vec![fill(2), hidden_fill], true, |coin| {
        coin == "ETH"
    });

    assert!(toasts.is_empty());
    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![2, 1]);
    assert_eq!(existing[1].coin, "ETH");
}

#[test]
fn live_fill_update_prepends_history_and_returns_visible_toasts() {
    let mut existing = vec![fill(3)];
    let mut sell_fill = fill(1);
    sell_fill.side = "A".to_string();
    let mut hidden_fill = fill(2);
    hidden_fill.coin = "ETH".to_string();

    let toast_fills =
        apply_fills_update(&mut existing, vec![sell_fill, hidden_fill], false, |coin| {
            coin == "ETH"
        });

    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![1, 2, 3]);
    assert_eq!(existing[1].coin, "ETH");
    assert_eq!(toast_fills.len(), 1);
    assert_eq!(toast_fills[0].coin, "BTC");
    assert_eq!(
        fill_toast_message(&toast_fills[0], &toast_fills[0].coin, &toast_fills[0].sz),
        "Filled SELL 0.1 BTC @ $100"
    );
}

#[test]
fn live_fill_update_deduplicates_existing_history_by_stable_identity() {
    let mut existing_fill = fill(3);
    existing_fill.tid = Some(77);
    let mut duplicate = fill(1);
    duplicate.tid = Some(77);
    duplicate.px = "101".to_string();
    let mut new_fill = fill(2);
    new_fill.tid = Some(78);
    let mut existing = vec![existing_fill];

    let toast_fills =
        apply_fills_update(&mut existing, vec![duplicate, new_fill], false, |_| false);

    let tids: Vec<Option<u64>> = existing.iter().map(|fill| fill.tid).collect();
    assert_eq!(tids, vec![Some(78), Some(77)]);
    assert_eq!(toast_fills.len(), 1);
    assert_eq!(toast_fills[0].tid, Some(78));
    assert_eq!(
        fill_toast_message(&toast_fills[0], &toast_fills[0].coin, &toast_fills[0].sz),
        "Filled BUY 0.1 BTC @ $100"
    );
}

#[test]
fn fill_snapshot_deduplicates_exact_hash_identity() {
    let mut first = fill(1);
    first.hash = Some("0xabc".to_string());
    let mut duplicate = first.clone();
    duplicate.hash = Some("0xabc".to_string());
    let mut existing = Vec::new();

    let toasts = apply_fills_update(&mut existing, vec![first, duplicate], true, |_| false);

    assert!(toasts.is_empty());
    assert_eq!(existing.len(), 1);
    assert_eq!(existing[0].time, 1);
}

#[test]
fn fill_snapshot_keeps_distinct_fills_from_same_transaction_hash() {
    let mut first = fill(1);
    first.hash = Some("0xabc".to_string());
    let mut second = fill(2);
    second.hash = Some("0xabc".to_string());
    second.px = "101".to_string();
    let mut existing = Vec::new();

    let toasts = apply_fills_update(&mut existing, vec![first, second], true, |_| false);

    assert!(toasts.is_empty());
    assert_eq!(existing.len(), 2);
    assert_eq!(
        existing.iter().map(|fill| fill.time).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn fill_toast_message_embeds_resolved_outcome_labels() {
    let mut outcome_fill = fill(1);
    outcome_fill.coin = "#950".to_string();
    outcome_fill.sz = "5.0".to_string();
    outcome_fill.px = "0.42".to_string();

    assert_eq!(
        fill_toast_message(&outcome_fill, "YES: Will BTC close green?", "5"),
        "Filled BUY 5 YES: Will BTC close green? @ $0.42"
    );
}

#[test]
fn chase_fill_summary_reports_weighted_fill_for_matching_oid() {
    assert_eq!(
        chase_fill_summary(
            &[
                fill_with_oid(1, 42, "100", "0.1"),
                fill_with_oid(2, 42, "110", "0.2"),
                fill_with_oid(3, 43, "1", "9"),
            ],
            42,
        ),
        Some("Chase filled: BUY 0.3 BTC @ $106.66666667 (oid 42)".to_string())
    );
}

#[test]
fn chase_fill_summary_deduplicates_matching_fills_by_stable_identity() {
    let mut first = fill_with_oid(1, 42, "100", "0.1");
    first.tid = Some(99);
    let mut duplicate = fill_with_oid(2, 42, "110", "0.1");
    duplicate.tid = Some(99);

    assert_eq!(
        chase_fill_summary(&[first, duplicate], 42),
        Some("Chase filled: BUY 0.1 BTC @ $100 (oid 42)".to_string())
    );
}

#[test]
fn chase_fill_summary_ignores_unmatched_or_unparseable_fills() {
    assert_eq!(
        chase_fill_summary(&[fill_with_oid(1, 43, "100", "1")], 42),
        None
    );
    assert_eq!(
        chase_fill_summary(&[fill_with_oid(1, 42, "bad", "1")], 42),
        Some("Chase filled (oid 42)".to_string())
    );
    assert_eq!(
        chase_fill_summary(
            &[
                fill_with_oid(1, 42, "NaN", "1"),
                fill_with_oid(2, 42, "100", "0"),
            ],
            42,
        ),
        Some("Chase filled (oid 42)".to_string())
    );
}
