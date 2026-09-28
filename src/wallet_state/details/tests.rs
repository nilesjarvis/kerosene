use super::*;
use crate::account::{
    AssetPosition, OpenOrder, SpotBalance, WalletDetailsData, WalletPositionDetail,
};
use crate::config::KeroseneConfig;
use serde_json::json;

const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
const OTHER: &str = "0xdef0000000000000000000000000000000000000";

fn terminal_with_windows() -> (TradingTerminal, [window::Id; 4]) {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.muted_tickers.insert("ETH".into());
    let ids = std::array::from_fn(|_| window::Id::unique());
    for (index, id) in ids.iter().enumerate() {
        let mut state =
            WalletDetailsWindowState::new(if index == 3 { OTHER } else { ADDRESS }.into());
        state.loading = index != 1;
        state.loading_context = Some(terminal.read_data_request_context());
        state.error = Some("previous error".into());
        state.last_refresh_ms = Some(42);
        if index != 2 {
            state.data = Some(snapshot());
        }
        terminal.wallet_detail_windows.insert(*id, state);
    }
    (terminal, ids)
}

fn snapshot() -> WalletDetailsData {
    WalletDetailsData {
        clearinghouse: serde_json::from_value(json!({
            "marginSummary": {
                "accountValue": "100", "totalNtlPos": "20", "totalMarginUsed": "10"
            },
            "withdrawable": "90", "assetPositions": []
        }))
        .expect("clearinghouse fixture"),
        spot: serde_json::from_value(json!({
            "balances": [], "portfolioMarginEnabled": true,
            "portfolioMarginRatio": "0.5",
            "tokenToAvailableAfterMaintenance": [[0, "80"]]
        }))
        .expect("spot fixture"),
        positions: vec![],
        open_orders: vec![
            order_detail("", "BTC", 1),
            order_detail("xyz", "xyz:OLD", 2),
            order_detail("flx", "flx:SOL", 3),
            order_detail("flx", "flx:ETH", 4),
        ],
        fills: vec![fill(1)],
        warnings: vec!["snapshot warning".into()],
        fetched_at_ms: 42,
    }
}

fn position(coin: &str) -> AssetPosition {
    serde_json::from_value(json!({"position": {
        "coin": coin, "szi": "1", "entryPx": "100", "positionValue": "110",
        "unrealizedPnl": "10", "leverage": {"type": "cross", "value": 2}
    }}))
    .expect("position fixture")
}

fn order_detail(dex: &str, coin: &str, oid: u64) -> WalletOpenOrderDetail {
    WalletOpenOrderDetail {
        dex: dex.into(),
        order: serde_json::from_value::<OpenOrder>(json!({
            "coin": coin, "side": "B", "limitPx": "100", "sz": "1",
            "oid": oid, "timestamp": 10
        }))
        .expect("order fixture"),
    }
}

fn balance(coin: &str) -> SpotBalance {
    serde_json::from_value(json!({
        "coin": coin, "total": "10", "hold": "2", "entryNtl": "8"
    }))
    .expect("balance fixture")
}

fn positions_update() -> WsUserData {
    let mut main = snapshot().clearinghouse;
    main.margin_summary.account_value = "200".into();
    main.cross_margin_summary = Some(main.margin_summary.clone());
    main.cross_maintenance_margin_used = Some("30".into());
    main.withdrawable = "180".into();
    main.asset_positions = vec![position("IGNORED")];
    WsUserData::AllDexPositions {
        main_state: Box::new(main),
        states_by_dex: Default::default(),
        all_positions: vec![position("BTC"), position("ETH"), position("xyz:TSLA")],
        position_details: vec![
            WalletPositionDetail {
                dex: "xyz".into(),
                asset_position: position("xyz:TSLA"),
            },
            WalletPositionDetail {
                dex: "".into(),
                asset_position: position("ETH"),
            },
            WalletPositionDetail {
                dex: "".into(),
                asset_position: position("BTC"),
            },
        ],
    }
}

#[test]
fn wallet_detail_stream_shares_timestamps_without_changing_loading_or_other_windows() {
    for event in [
        positions_update(),
        WsUserData::OpenOrders {
            dex: "xyz".into(),
            orders: vec![],
        },
        WsUserData::SpotBalances(vec![]),
        WsUserData::Fills {
            fills: vec![],
            is_snapshot: false,
        },
        WsUserData::Fills {
            fills: vec![],
            is_snapshot: true,
        },
    ] {
        let (mut terminal, ids) = terminal_with_windows();
        let context = terminal.read_data_request_context();
        let connected = terminal.connected_address.clone();
        let before = TradingTerminal::now_ms();
        let _ = terminal.apply_wallet_details_ws_update(
            Some(format!("  {}  ", ADDRESS.to_ascii_uppercase())),
            event,
        );
        let after = TradingTerminal::now_ms();
        let timestamp = terminal.wallet_detail_windows[&ids[0]]
            .last_refresh_ms
            .expect("updated timestamp");
        assert!((before..=after).contains(&timestamp));
        for (index, id) in ids.iter().enumerate() {
            let state = &terminal.wallet_detail_windows[id];
            assert_eq!(state.loading, index != 1);
            assert_eq!(state.loading_context, Some(context));
            assert_eq!(
                state.last_refresh_ms,
                Some(if index == 3 { 42 } else { timestamp })
            );
            assert_eq!(
                state.error.as_deref(),
                if index == 3 {
                    Some("previous error")
                } else {
                    None
                }
            );
            if let Some(data) = &state.data {
                assert_eq!(data.fetched_at_ms, if index == 3 { 42 } else { timestamp });
                assert_eq!(data.warnings, ["snapshot warning"]);
            } else {
                assert_eq!(index, 2);
            }
        }
        assert_eq!(terminal.connected_address, connected);
        let other = terminal.wallet_detail_windows[&ids[3]]
            .data
            .as_ref()
            .expect("other snapshot");
        assert_eq!(other.open_orders.len(), 4);
        assert_eq!(other.fills.len(), 1);
        assert!(other.positions.is_empty());
    }
}

#[test]
fn wallet_detail_positions_preserve_source_order_margin_fields_and_independent_snapshots() {
    let (mut terminal, ids) = terminal_with_windows();
    let _ = terminal.apply_wallet_details_ws_update(Some(ADDRESS.into()), positions_update());
    for id in &ids[..2] {
        let data = terminal.wallet_detail_windows[id]
            .data
            .as_ref()
            .expect("snapshot");
        let clearinghouse = &data.clearinghouse;
        assert_eq!(clearinghouse.margin_summary.account_value, "200");
        assert_eq!(clearinghouse.margin_summary.total_ntl_pos, "20");
        assert_eq!(clearinghouse.margin_summary.total_margin_used, "10");
        assert_eq!(clearinghouse.withdrawable, "180");
        assert_eq!(
            clearinghouse
                .cross_margin_summary
                .as_ref()
                .expect("cross margin")
                .account_value,
            "200"
        );
        assert_eq!(
            clearinghouse.cross_maintenance_margin_used.as_deref(),
            Some("30")
        );
        assert_eq!(
            clearinghouse
                .asset_positions
                .iter()
                .map(|p| p.position.coin.as_str())
                .collect::<Vec<_>>(),
            ["BTC", "xyz:TSLA"]
        );
        assert_eq!(
            data.positions
                .iter()
                .map(|p| (p.dex.as_str(), p.asset_position.position.coin.as_str()))
                .collect::<Vec<_>>(),
            [("xyz", "xyz:TSLA"), ("", "BTC")]
        );
        assert_eq!(data.open_orders.len(), 4);
        assert!(data.spot.balances.is_empty());
        assert_eq!(data.fills.len(), 1);
    }
    terminal
        .wallet_detail_windows
        .get_mut(&ids[0])
        .expect("window")
        .data
        .as_mut()
        .expect("snapshot")
        .positions
        .clear();
    assert_eq!(
        terminal.wallet_detail_windows[&ids[1]]
            .data
            .as_ref()
            .expect("snapshot")
            .positions
            .len(),
        2
    );
}

#[test]
fn wallet_detail_orders_replace_one_dex_normalize_names_and_remove_hidden_rows() {
    let (mut terminal, ids) = terminal_with_windows();
    let _ = terminal.apply_wallet_details_ws_update(
        Some(ADDRESS.into()),
        WsUserData::OpenOrders {
            dex: "xyz".into(),
            orders: vec![
                order_detail("", "TSLA", 5).order,
                order_detail("", "ETH", 6).order,
                order_detail("", "xyz:AAPL", 7).order,
            ],
        },
    );
    for id in &ids[..2] {
        let data = terminal.wallet_detail_windows[id]
            .data
            .as_ref()
            .expect("snapshot");
        assert_eq!(
            data.open_orders
                .iter()
                .map(|o| (o.dex.as_str(), o.order.coin.as_str(), o.order.oid))
                .collect::<Vec<_>>(),
            [
                ("", "BTC", 1),
                ("flx", "flx:SOL", 3),
                ("xyz", "xyz:TSLA", 5),
                ("xyz", "xyz:AAPL", 7)
            ]
        );
        assert_eq!(data.clearinghouse.margin_summary.account_value, "100");
        assert_eq!(data.fills.len(), 1);
    }
}

#[test]
fn wallet_detail_spot_updates_replace_balances_but_preserve_margin_metadata() {
    let (mut terminal, ids) = terminal_with_windows();
    let _ = terminal.apply_wallet_details_ws_update(
        Some(ADDRESS.into()),
        WsUserData::SpotBalances(vec![balance("USDC"), balance("ETH"), balance("BTC")]),
    );
    for id in &ids[..2] {
        let data = terminal.wallet_detail_windows[id]
            .data
            .as_ref()
            .expect("snapshot");
        assert_eq!(
            data.spot
                .balances
                .iter()
                .map(|b| b.coin.as_str())
                .collect::<Vec<_>>(),
            ["USDC", "BTC"]
        );
        assert!(data.spot.portfolio_margin_enabled);
        assert_eq!(data.spot.portfolio_margin_ratio.as_deref(), Some("0.5"));
        assert_eq!(
            data.spot.token_to_available_after_maintenance,
            Some(vec![(0, "80".into())])
        );
        assert_eq!(data.open_orders.len(), 4);
    }
}

#[test]
fn wallet_detail_fill_events_keep_hidden_fills_and_first_occurrence_in_each_window() {
    for is_snapshot in [false, true] {
        let (mut terminal, ids) = terminal_with_windows();
        let mut incoming = fill(2);
        incoming.coin = "ETH".into();
        let mut duplicate = incoming.clone();
        duplicate.px = "200".into();
        let _ = terminal.apply_wallet_details_ws_update(
            Some(ADDRESS.into()),
            WsUserData::Fills {
                fills: vec![incoming, duplicate, fill(3)],
                is_snapshot,
            },
        );
        for id in &ids[..2] {
            let data = terminal.wallet_detail_windows[id]
                .data
                .as_ref()
                .expect("snapshot");
            let expected = if is_snapshot {
                vec![Some(2), Some(3)]
            } else {
                vec![Some(1), Some(2), Some(3)]
            };
            assert_eq!(
                data.fills.iter().map(|f| f.tid).collect::<Vec<_>>(),
                expected
            );
            let hidden = data
                .fills
                .iter()
                .find(|f| f.tid == Some(2))
                .expect("hidden fill retained");
            assert_eq!(hidden.coin, "ETH");
            assert_eq!(hidden.px, "100");
            assert_eq!(data.open_orders.len(), 4);
        }
    }
}

#[test]
fn wallet_detail_invalid_sources_ignore_private_data_but_all_mids_are_address_independent() {
    let (mut terminal, ids) = terminal_with_windows();
    for address in [None, Some("invalid".into())] {
        let _ = terminal.apply_wallet_details_ws_update(address, positions_update());
    }
    for (index, address) in [None, Some("invalid".into()), Some(ADDRESS.into())]
        .into_iter()
        .enumerate()
    {
        let price = 100.0 + index as f64;
        let _ = terminal.apply_wallet_details_ws_update(
            address,
            WsUserData::AllMids(std::collections::HashMap::from([("BTC".into(), price)])),
        );
        assert_eq!(terminal.all_mids.get("BTC"), Some(&price));
    }
    for id in ids {
        let state = &terminal.wallet_detail_windows[&id];
        assert_eq!(state.last_refresh_ms, Some(42));
        assert_eq!(state.error.as_deref(), Some("previous error"));
        if let Some(data) = &state.data {
            assert!(data.positions.is_empty());
            assert_eq!(data.fetched_at_ms, 42);
        }
    }
}

#[test]
fn wallet_detail_lag_preserves_pending_context_and_timestamps_while_refreshing_idle_windows() {
    let (mut terminal, ids) = terminal_with_windows();
    let old_context = terminal.read_data_request_context();
    terminal.read_data_provider_generation += 1;
    let current_context = terminal.read_data_request_context();
    let _ = terminal
        .apply_wallet_details_ws_update(Some(ADDRESS.into()), WsUserData::Lagged { skipped: 9 });
    for (index, id) in ids.iter().enumerate() {
        let state = &terminal.wallet_detail_windows[id];
        assert!(state.loading);
        assert_eq!(
            state.loading_context,
            Some(if index == 1 {
                current_context
            } else {
                old_context
            })
        );
        assert_eq!(state.last_refresh_ms, Some(42));
        assert_eq!(
            state.error.as_deref(),
            Some(if index == 3 {
                "previous error"
            } else {
                "Wallet detail stream lagged (9 updates skipped); refreshing snapshot"
            })
        );
        if let Some(data) = &state.data {
            assert_eq!(data.fetched_at_ms, 42);
        }
    }
}

#[test]
fn wallet_detail_fill_snapshot_replaces_existing_fills() {
    let mut existing = vec![fill(1)];

    merge_wallet_detail_fills(&mut existing, &[fill(2)], true);

    assert_eq!(existing.len(), 1);
    assert_eq!(existing[0].tid, Some(2));
}

#[test]
fn wallet_detail_incremental_fills_are_deduplicated() {
    let mut existing = vec![fill(1)];

    merge_wallet_detail_fills(&mut existing, &[fill(1), fill(2)], false);

    assert_eq!(existing.len(), 2);
    assert_eq!(existing[0].tid, Some(1));
    assert_eq!(existing[1].tid, Some(2));
}

fn fill(tid: u64) -> UserFill {
    UserFill {
        coin: "BTC".to_string(),
        px: "100".to_string(),
        sz: "0.1".to_string(),
        side: "B".to_string(),
        time: tid,
        hash: None,
        tid: Some(tid),
        oid: Some(tid),
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0".to_string(),
        fee_token: None,
        start_position: None,
    }
}
