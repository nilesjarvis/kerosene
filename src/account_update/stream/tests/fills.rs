use super::super::{apply_fills_update, chase_fill_summary, prepend_recent_fills};
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
fn fill_snapshot_replaces_existing_history_and_filters_muted_symbols() {
    let mut existing = vec![fill(10)];
    let mut muted_fill = fill(1);
    muted_fill.coin = "ETH".to_string();

    let toasts = apply_fills_update(&mut existing, vec![fill(2), muted_fill], true, |coin| {
        coin == "ETH"
    });

    assert!(toasts.is_empty());
    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![2]);
}

#[test]
fn live_fill_update_prepends_history_and_returns_toasts() {
    let mut existing = vec![fill(3)];
    let mut sell_fill = fill(1);
    sell_fill.side = "A".to_string();

    let toasts = apply_fills_update(&mut existing, vec![sell_fill, fill(2)], false, |_| false);

    let times: Vec<u64> = existing.iter().map(|fill| fill.time).collect();
    assert_eq!(times, vec![1, 2, 3]);
    assert_eq!(
        toasts,
        vec![
            "Filled SELL 0.1 BTC @ $100".to_string(),
            "Filled BUY 0.1 BTC @ $100".to_string(),
        ]
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
