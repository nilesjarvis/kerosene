use super::*;
use crate::account::{OpenOrder, UserFill};

fn open_order(oid: u64, reduce_only: Option<bool>) -> OpenOrder {
    OpenOrder {
        coin: "BTC".to_string(),
        side: "B".to_string(),
        limit_px: "100".to_string(),
        sz: "0.1".to_string(),
        oid,
        timestamp: 1,
        reduce_only,
    }
}

fn fill(time: u64) -> UserFill {
    UserFill {
        coin: "BTC".to_string(),
        px: "100".to_string(),
        sz: "0.1".to_string(),
        side: "B".to_string(),
        time,
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0.01".to_string(),
    }
}

#[test]
fn websocket_open_order_preserves_known_reduce_only_metadata_when_omitted() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(42, None);

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, Some(true));
}

#[test]
fn websocket_open_order_keeps_unknown_reduce_only_for_new_orders() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(43, None);

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, None);
}

#[test]
fn websocket_open_order_keeps_explicit_reduce_only_metadata() {
    let existing = vec![open_order(42, Some(true))];
    let mut incoming = open_order(42, Some(false));

    preserve_open_order_reduce_only(&mut incoming, &existing);

    assert_eq!(incoming.reduce_only, Some(false));
}

#[test]
fn websocket_account_repair_skips_when_initial_fetch_is_loading() {
    assert!(!should_repair_account_from_ws(
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        false,
        true,
    ));
    assert!(should_repair_account_from_ws(
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        false,
        false,
    ));
    assert!(!should_repair_account_from_ws(
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        true,
        false,
    ));
    assert!(!should_repair_account_from_ws(None, false, false));
}

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
