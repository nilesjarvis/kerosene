use super::*;
use crate::account::{OpenOrder, UserFill};
use crate::signing::ChaseOrder;
use std::time::Instant;

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
        oid: None,
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0.01".to_string(),
    }
}

fn fill_with_oid(time: u64, oid: u64, px: &str, sz: &str) -> UserFill {
    let mut fill = fill(time);
    fill.oid = Some(oid);
    fill.px = px.to_string();
    fill.sz = sz.to_string();
    fill
}

fn chase_order() -> ChaseOrder {
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        remaining_size: 1.0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: Instant::now(),
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: false,
        missing_open_order_refresh_requested: true,
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
fn hip3_open_order_stream_symbols_are_normalized() {
    let mut orders = vec![open_order(42, Some(false)), open_order(43, Some(false))];
    orders[1].coin = "flx:ETH".to_string();

    normalize_dex_open_order_coins("flx", &mut orders);

    assert_eq!(orders[0].coin, "flx:BTC");
    assert_eq!(orders[1].coin, "flx:ETH");
}

#[test]
fn main_dex_open_order_stream_symbols_stay_unprefixed() {
    let mut orders = vec![open_order(42, Some(false))];

    normalize_dex_open_order_coins("", &mut orders);

    assert_eq!(orders[0].coin, "BTC");
}

#[test]
fn open_order_sync_updates_chase_size_price_and_confirmation() {
    let mut chase = chase_order();
    let mut order = open_order(42, Some(false));
    order.sz = "0.25".to_string();
    order.limit_px = "101.5".to_string();

    assert_eq!(
        apply_open_order_to_chase(&mut chase, &order, ChaseOpenOrderPriceSync::Trust),
        Ok(())
    );

    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 101.5);
    assert_eq!(chase.current_price_wire, "101.5");
    assert!(chase.oid_confirmed);
    assert!(!chase.missing_open_order_refresh_requested);
}

#[test]
fn open_order_sync_rejects_invalid_remaining_size() {
    let mut chase = chase_order();
    let mut order = open_order(42, Some(false));
    order.sz = "0".to_string();

    assert_eq!(
        apply_open_order_to_chase(&mut chase, &order, ChaseOpenOrderPriceSync::Trust),
        Err(())
    );
    assert_eq!(chase.remaining_size, 1.0);
}

#[test]
fn open_order_sync_preserves_expected_price_until_modify_confirmation_catches_up() {
    let mut chase = chase_order();
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase.oid_confirmed = false;

    let mut stale_order = open_order(42, Some(false));
    stale_order.sz = "0.25".to_string();
    stale_order.limit_px = "100".to_string();

    assert_eq!(
        apply_open_order_to_chase(
            &mut chase,
            &stale_order,
            ChaseOpenOrderPriceSync::PreserveExpectedIfUnconfirmed,
        ),
        Ok(())
    );

    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 101.0);
    assert_eq!(chase.current_price_wire, "101");
    assert!(chase.oid_confirmed);
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
}
