use super::*;
use crate::account::{WalletDetailsData, WalletPositionDetail};
use crate::config::{AccountProfile, KeroseneConfig};
use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember, WalletClusterMemberData};
use crate::wallet_state::AddressBookEntry;
use serde_json::json;

const SECOND: &str = "0x2222222222222222222222222222222222222222";

fn position(coin: &str, size: &str, value: &str, pnl: &str) -> WalletPositionDetail {
    WalletPositionDetail {
        dex: "xyz".into(),
        asset_position: serde_json::from_value(json!({"position": {
            "coin": coin, "szi": size, "entryPx": "10", "positionValue": value,
            "unrealizedPnl": pnl, "leverage": {"type": "cross", "value": 2}
        }}))
        .expect("position fixture"),
    }
}

fn details(positions: Vec<WalletPositionDetail>) -> WalletDetailsData {
    WalletDetailsData {
        clearinghouse: serde_json::from_value(json!({
            "marginSummary": {"accountValue": "0", "totalNtlPos": "0", "totalMarginUsed": "0"},
            "withdrawable": "0", "assetPositions": []
        }))
        .expect("clearinghouse fixture"),
        spot: serde_json::from_value(json!({"balances": []})).expect("spot fixture"),
        positions,
        open_orders: vec![],
        fills: vec![],
        warnings: vec![],
        fetched_at_ms: 42,
    }
}

fn terminal_with_positions(positions: Vec<WalletPositionDetail>) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.wallet_clusters.clusters = vec![WalletCluster {
        id: "cluster".into(),
        name: "Cluster".into(),
        members: ["missing", "first", "second"]
            .into_iter()
            .map(|id| WalletClusterMember {
                profile_secret_id: id.into(),
                weight: 0.0,
                weight_input: "0".into(),
            })
            .collect(),
    }];
    terminal.wallet_clusters.selected_cluster_id = Some("cluster".into());
    terminal.accounts = vec![AccountProfile {
        secret_id: "first".into(),
        name: " First ".into(),
        wallet_address: ADDRESS.into(),
        master_address: None,
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }];
    terminal.address_book.insert(
        SECOND.into(),
        AddressBookEntry {
            label: "Second".into(),
            ..Default::default()
        },
    );
    terminal.wallet_clusters.member_data.insert(
        "first".into(),
        WalletClusterMemberData {
            address: ADDRESS.into(),
            data: Some(details(positions)),
            ..Default::default()
        },
    );
    terminal
}

#[test]
fn cluster_summaries_preserve_group_order_missing_values_labels_and_owned_results() {
    let mut terminal = terminal_with_positions(vec![
        position("BTC", "2", "100", "10"),
        position("ETH", "1", "20", "2"),
        position("BTC", "-1", "NaN", "NaN"),
        position("BTC", "0.5", "20", "3"),
        position("SOL", "-2", "invalid", "NaN"),
        position("ZERO", "0", "100", "10"),
        position("TINY", "1e-12", "100", "10"),
        position("INVALID", "NaN", "100", "10"),
    ]);
    terminal.all_mids.insert("SOL".into(), 50.0);
    terminal
        .all_mids_updated_at_ms
        .insert("SOL".into(), TradingTerminal::now_ms());
    terminal.wallet_clusters.member_data.insert(
        "second".into(),
        WalletClusterMemberData {
            address: SECOND.into(),
            data: Some(details(vec![
                position("ETH", "1", "0", "NaN"),
                position("TIE", "1", "20", "1"),
            ])),
            ..Default::default()
        },
    );
    let summaries = terminal.wallet_cluster_position_summaries();
    assert_eq!(
        summaries
            .iter()
            .map(|s| s.symbol.as_str())
            .collect::<Vec<_>>(),
        ["SOL", "BTC", "ETH", "TIE"]
    );
    assert_eq!(summaries[0].value, Some(100.0));
    assert_eq!(summaries[0].unrealized_pnl, None);
    let btc = &summaries[1];
    assert_eq!(
        (btc.net_size, btc.long_size, btc.short_size),
        (1.5, 2.5, 1.0)
    );
    assert_eq!((btc.value, btc.unrealized_pnl), (Some(20.0), Some(3.0)));
    assert_eq!(
        btc.members.iter().map(|m| m.size).collect::<Vec<_>>(),
        [2.0, -1.0, 0.5]
    );
    assert_eq!(btc.members[0].entry_price, Some(10.0));
    assert_eq!(btc.members[0].dex, "xyz");
    assert_eq!(btc.members[0].label, "First");
    assert_eq!(btc.members[0].address, ADDRESS);
    assert_eq!(btc.members[0].profile_secret_id, "first");
    assert_eq!(btc.members[1].value, None);
    let eth = &summaries[2];
    assert_eq!(
        (eth.net_size, eth.long_size, eth.short_size),
        (2.0, 2.0, 0.0)
    );
    assert_eq!((eth.value, eth.unrealized_pnl), (Some(20.0), None));
    assert_eq!(eth.members[1].label, "Second");
    assert_eq!(eth.members[1].address, SECOND);
    terminal.wallet_clusters.member_data.clear();
    assert_eq!(summaries[1].members.len(), 3);
}

#[test]
fn cluster_summaries_keep_floating_point_accumulation_order_and_empty_selection_behavior() {
    let mut terminal = terminal_with_positions(vec![
        position("BTC", "1e16", "1", "1"),
        position("BTC", "1", "1", "1"),
        position("BTC", "-1e16", "1", "1"),
    ]);
    let summaries = terminal.wallet_cluster_position_summaries();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].net_size, 0.0);
    assert_eq!(summaries[0].value, Some(3.0));
    assert_eq!(summaries[0].unrealized_pnl, Some(3.0));
    terminal.wallet_clusters.selected_cluster_id = None;
    assert!(terminal.wallet_cluster_position_summaries().is_empty());
}
