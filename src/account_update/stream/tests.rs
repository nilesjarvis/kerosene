use super::*;
use crate::account::{
    AssetPosition, ClearinghouseState, MarginSummary, Position, PositionLeverage,
};

mod chase_fills;
mod chase_reprice;
mod chase_stop;
mod fills;
mod fixtures;
mod orders;

use fixtures::{account_data_with_timestamp, open_order};

fn test_clearinghouse_state() -> ClearinghouseState {
    ClearinghouseState {
        margin_summary: MarginSummary {
            account_value: "0".to_string(),
            total_ntl_pos: "0".to_string(),
            total_margin_used: "0".to_string(),
        },
        cross_margin_summary: None,
        cross_maintenance_margin_used: None,
        withdrawable: "0".to_string(),
        asset_positions: Vec::new(),
    }
}

fn test_position(coin: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: "1".to_string(),
            entry_px: "100".to_string(),
            position_value: "100".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 1,
            },
            margin_used: "0".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
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
fn lagged_connected_user_stream_queues_followup_when_refresh_in_flight() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.account_loading = true;
    terminal.account_refresh_followup_pending = false;
    terminal.account_reconciliation_required = false;

    let _task = terminal.apply_ws_user_data_update(
        terminal.connected_address.clone(),
        WsUserData::Lagged { skipped: 3 },
    );

    assert!(terminal.account_loading);
    assert!(terminal.account_refresh_followup_pending);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn non_position_ws_updates_do_not_refresh_position_snapshot_timestamp() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data_address = Some(address.clone());
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
fn position_ws_update_keeps_hidden_exposure_in_account_snapshot() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.muted_tickers.insert("ETH".to_string());
    terminal.account_data_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::AllDexPositions {
            main_state: Box::new(test_clearinghouse_state()),
            states_by_dex: std::collections::HashMap::new(),
            all_positions: vec![test_position("BTC"), test_position("ETH")],
            position_details: Vec::new(),
        },
    );

    let coins: Vec<_> = terminal
        .account_data
        .as_ref()
        .expect("account data")
        .clearinghouse
        .asset_positions
        .iter()
        .map(|position| position.position.coin.as_str())
        .collect();
    assert_eq!(coins, vec!["BTC", "ETH"]);
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
fn websocket_account_repair_respects_account_refresh_backoff() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data = None;
    terminal.account_loading = false;
    terminal.account_refresh_backoff_until_ms = Some(TradingTerminal::now_ms() + 60_000);

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: Vec::new(),
        },
    );

    assert!(!terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    assert!(
        terminal
            .account_error
            .as_deref()
            .is_some_and(|error| error.contains("rate limited"))
    );
}

#[test]
fn websocket_delta_clears_mismatched_account_snapshot_and_repairs() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    terminal.account_data = Some(account_data_with_timestamp(1_000));
    terminal.account_loading = false;

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![open_order(42, Some(false))],
        },
    );

    assert!(terminal.account_data.is_none());
    assert_eq!(terminal.account_data_address, None);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}

#[test]
fn ws_fill_consumes_market_indicator_before_rest_ack() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.optimistic_account_updates = true;
    terminal.account_data_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1));
    let pending_id = terminal.add_pending_market_order_placement_indicator(
        address.clone(),
        "BTC".to_string(),
        true,
        "0.1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());
    let fill_time = TradingTerminal::now_ms() + 50;

    // The websocket delivers the fill while the REST place ack is still in
    // flight; the projection must collapse instead of double-counting.
    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::Fills {
            fills: vec![fixtures::fill(fill_time)],
            is_snapshot: false,
        },
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert_eq!(terminal.optimistic_position_delta_for_symbol("BTC"), None);
}

#[test]
fn ws_fill_update_preserves_canonical_market_symbols_and_wire_sides_in_account_data() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1));
    let mut native = fixtures::fill(1);
    native.coin = "BTC".to_string();
    native.side = "B".to_string();
    let mut hip3 = fixtures::fill(2);
    hip3.coin = "flx:BTC".to_string();
    hip3.side = "B".to_string();
    let mut spot = fixtures::fill(3);
    spot.coin = "@107".to_string();
    spot.side = "A".to_string();
    let mut outcome = fixtures::fill(4);
    outcome.coin = "#950".to_string();
    outcome.side = "A".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::Fills {
            fills: vec![native, hip3, spot, outcome],
            is_snapshot: true,
        },
    );

    let data = terminal
        .account_data
        .as_ref()
        .expect("account data should remain loaded");
    let parsed: Vec<(&str, &str)> = data
        .fills
        .iter()
        .map(|fill| (fill.coin.as_str(), fill.side.as_str()))
        .collect();
    assert_eq!(
        parsed,
        vec![("BTC", "B"), ("flx:BTC", "B"), ("@107", "A"), ("#950", "A")]
    );
}

#[test]
fn fill_toast_size_label_formats_outcome_fills_as_whole_contracts() {
    let (terminal, _) = TradingTerminal::boot();
    let mut outcome_fill = fixtures::fill(1);
    outcome_fill.coin = "#950".to_string();
    outcome_fill.sz = "5.0".to_string();
    assert_eq!(terminal.fill_toast_size_label(&outcome_fill), "5");

    outcome_fill.sz = "bad".to_string();
    assert_eq!(terminal.fill_toast_size_label(&outcome_fill), "bad");

    let perp_fill = fixtures::fill(2);
    assert_eq!(terminal.fill_toast_size_label(&perp_fill), "0.1");
}

#[test]
fn ws_outcome_fill_toast_uses_display_label_and_whole_contract_size() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1));
    terminal
        .outcome_display_labels
        .insert("#950".to_string(), "YES: Will BTC close green?".to_string());
    let mut outcome_fill = fixtures::fill(1);
    outcome_fill.coin = "#950".to_string();
    outcome_fill.sz = "5.0".to_string();
    outcome_fill.px = "0.42".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::Fills {
            fills: vec![outcome_fill],
            is_snapshot: false,
        },
    );

    assert!(
        terminal
            .toasts
            .iter()
            .any(|toast| toast.message == "Filled BUY 5 YES: Will BTC close green? @ $0.42"),
        "fill toast must resolve the outcome label and whole-contract size"
    );
}

#[test]
fn ws_fill_snapshot_does_not_consume_market_indicators() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1));
    let pending_id = terminal.add_pending_market_order_placement_indicator(
        address.clone(),
        "BTC".to_string(),
        true,
        "0.1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());
    let fill_time = TradingTerminal::now_ms() + 50;

    // Snapshots replay history; only live fill deltas consume projections.
    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::Fills {
            fills: vec![fixtures::fill(fill_time)],
            is_snapshot: true,
        },
    );

    assert_eq!(terminal.pending_order_indicators.len(), 1);
}
