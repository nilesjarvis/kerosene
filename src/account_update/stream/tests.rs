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
fn position_ws_update_keeps_hidden_exposure_in_account_snapshot() {
    let (mut terminal, _) = TradingTerminal::boot();
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.muted_tickers.insert("ETH".to_string());
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
    assert!(!terminal.account_reconciliation_required);
    assert!(
        terminal
            .account_error
            .as_deref()
            .is_some_and(|error| error.contains("rate limited"))
    );
}
