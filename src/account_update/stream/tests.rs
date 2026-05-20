use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary, OpenOrder,
    SpotClearinghouseState, UserFill,
};
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
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: Instant::now(),
        started_at_ms: 1_000,
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        pending_size_correction: false,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: false,
        missing_open_order_refresh_requested: true,
    }
}

fn account_data_with_timestamp(fetched_at_ms: u64) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "0".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "0".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms,
    }
}

#[test]
fn lagged_connected_user_stream_marks_account_loading_immediately() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;

    let _task = terminal.apply_ws_user_data_update(
        terminal.connected_address.clone(),
        WsUserData::Lagged { skipped: 3 },
    );

    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    assert_eq!(terminal.account_error, None);
}

#[test]
fn non_position_ws_updates_do_not_refresh_position_snapshot_timestamp() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![open_order(42, Some(false))],
        },
    );

    assert_eq!(
        terminal
            .account_data
            .as_ref()
            .map(|data| data.fetched_at_ms),
        Some(1_000)
    );
}

#[test]
fn lagged_non_connected_user_stream_does_not_mark_main_account_loading() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;

    let _task = terminal.apply_ws_user_data_update(
        Some("0xdef0000000000000000000000000000000000000".to_string()),
        WsUserData::Lagged { skipped: 3 },
    );

    assert!(!terminal.account_loading);
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
        Ok(false)
    );

    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 101.5);
    assert_eq!(chase.current_price_wire, "101.5");
    assert!(chase.oid_confirmed);
    assert!(!chase.pending_size_correction);
    assert!(!chase.missing_open_order_refresh_requested);
}

#[test]
fn open_order_sync_clamps_chase_size_to_unfilled_target() {
    let mut chase = chase_order();
    chase.filled_size = 0.9;
    let mut order = open_order(42, Some(false));
    order.sz = "0.2".to_string();

    assert_eq!(
        apply_open_order_to_chase(&mut chase, &order, ChaseOpenOrderPriceSync::Trust),
        Ok(true)
    );

    assert!((chase.remaining_size - 0.1).abs() < 1e-12);
    assert!(chase.pending_size_correction);
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
fn chase_fill_reconciliation_updates_filled_and_remaining_size() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase_order());
    let mut data = account_data_with_timestamp(1_000);
    data.fills = vec![fill_with_oid(1_001, 42, "100", "0.1")];
    terminal.account_data = Some(data);

    terminal.reconcile_chase_fills_from_account();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert!((chase.filled_size - 0.1).abs() < f64::EPSILON);
    assert!((chase.remaining_size - 0.9).abs() < f64::EPSILON);
}

#[test]
fn chase_fill_reconciliation_removes_fully_filled_chase() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase_order());
    let mut data = account_data_with_timestamp(1_000);
    data.fills = vec![fill_with_oid(1_001, 42, "100", "1.0")];
    terminal.account_data = Some(data);

    terminal.reconcile_chase_fills_from_account();

    assert!(terminal.chase_orders.is_empty());
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| !*is_error && message.contains("Chase filled"))
    );
}

#[test]
fn chase_fill_reconciliation_sums_known_reprice_oids() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase_order();
    chase.known_oids.push(43);
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    data.fills = vec![
        fill_with_oid(1_001, 42, "100", "0.1"),
        fill_with_oid(1_002, 43, "101", "0.2"),
    ];
    terminal.account_data = Some(data);

    terminal.reconcile_chase_fills_from_account();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert!((chase.filled_size - 0.3).abs() < 1e-12);
    assert!((chase.remaining_size - 0.7).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_counts_matching_oids_before_local_chase_start() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase_order();
    chase.started_at_ms = 1_000;
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    data.fills = vec![
        fill_with_oid(900, 42, "100", "0.4"),
        fill_with_oid(1_001, 42, "101", "0.1"),
    ];
    terminal.account_data = Some(data);

    terminal.reconcile_chase_fills_from_account();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert!((chase.filled_size - 0.5).abs() < 1e-12);
    assert!((chase.remaining_size - 0.5).abs() < 1e-12);
}

#[test]
fn chase_fill_reconciliation_deduplicates_matching_oid_fills() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase_order();
    chase.started_at_ms = 1_000;
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    let duplicate = fill_with_oid(900, 42, "100", "0.4");
    data.fills = vec![
        duplicate.clone(),
        duplicate,
        fill_with_oid(1_001, 42, "101", "0.1"),
    ];
    terminal.account_data = Some(data);

    terminal.reconcile_chase_fills_from_account();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert!((chase.filled_size - 0.5).abs() < 1e-12);
    assert!((chase.remaining_size - 0.5).abs() < 1e-12);
}

#[test]
fn chase_reprice_reconciliation_pauses_on_incomplete_account_snapshot() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    let mut chase = chase_order();
    chase.pending_best_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    data.completeness.fills_complete = false;
    terminal.account_data = Some(data);

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = terminal
        .chase_orders
        .get(&1)
        .expect("chase should remain paused");
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.current_oid, Some(42));
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error && message.contains("Chase paused"))
    );
}

#[test]
fn chase_reprice_reconciliation_clears_confirmed_pending_target() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase_order();
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase.pending_best_price = Some(101.0);
    chase.oid_confirmed = false;
    chase.missing_open_order_refresh_requested = true;
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    let mut order = open_order(42, Some(false));
    order.limit_px = "101".to_string();
    data.open_orders = vec![order];
    terminal.account_data = Some(data);

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.pending_op, None);
    assert_eq!(chase.pending_best_price, None);
    assert!(!chase.pending_size_correction);
    assert!(chase.oid_confirmed);
}

#[test]
fn open_order_sync_preserves_expected_price_until_modify_confirmation_catches_up() {
    let mut chase = chase_order();
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase.oid_confirmed = false;
    chase.pending_best_price = Some(101.0);

    let mut stale_order = open_order(42, Some(false));
    stale_order.sz = "0.25".to_string();
    stale_order.limit_px = "100".to_string();

    assert_eq!(
        apply_open_order_to_chase(
            &mut chase,
            &stale_order,
            ChaseOpenOrderPriceSync::PreserveExpectedIfUnconfirmed,
        ),
        Ok(false)
    );

    assert_eq!(chase.remaining_size, 0.25);
    assert_eq!(chase.current_price, 101.0);
    assert_eq!(chase.current_price_wire, "101");
    assert!(!chase.oid_confirmed);
    assert!(!chase.pending_size_correction);
    assert_eq!(chase.pending_best_price, Some(101.0));

    let mut confirmed_order = open_order(42, Some(false));
    confirmed_order.sz = "0.25".to_string();
    confirmed_order.limit_px = "101".to_string();

    assert_eq!(
        apply_open_order_to_chase(
            &mut chase,
            &confirmed_order,
            ChaseOpenOrderPriceSync::PreserveExpectedIfUnconfirmed,
        ),
        Ok(false)
    );

    assert_eq!(chase.current_price, 101.0);
    assert_eq!(chase.current_price_wire, "101");
    assert!(chase.oid_confirmed);
    assert!(!chase.pending_size_correction);
    assert_eq!(chase.pending_best_price, None);
}

#[test]
fn websocket_open_order_confirmation_clears_refreshing_chase_status() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1_000));
    terminal.order_status = Some((
        "Chasing (oid 42); refreshing account data...".to_string(),
        false,
    ));

    let mut chase = chase_order();
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase.oid_confirmed = false;
    chase.missing_open_order_refresh_requested = true;
    terminal.chase_orders.insert(1, chase);

    let mut order = open_order(42, Some(false));
    order.limit_px = "101".to_string();
    order.sz = "1.0".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![order],
        },
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert!(chase.oid_confirmed);
    assert!(!chase.missing_open_order_refresh_requested);
    assert_eq!(
        terminal.order_status,
        Some(("Chasing (oid 42)...".to_string(), false))
    );
}

#[test]
fn stale_websocket_open_order_keeps_chase_refresh_pending() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1_000));
    terminal.order_status = Some((
        "Chasing (oid 42); refreshing account data...".to_string(),
        false,
    ));

    let mut chase = chase_order();
    chase.current_price = 101.0;
    chase.current_price_wire = "101".to_string();
    chase.oid_confirmed = false;
    chase.missing_open_order_refresh_requested = true;
    terminal.chase_orders.insert(1, chase);

    let mut stale_order = open_order(42, Some(false));
    stale_order.limit_px = "100".to_string();
    stale_order.sz = "1.0".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![stale_order],
        },
    );

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert!(!chase.oid_confirmed);
    assert!(chase.missing_open_order_refresh_requested);
    assert_eq!(
        terminal.order_status,
        Some((
            "Chasing (oid 42); refreshing account data...".to_string(),
            false
        ))
    );
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
