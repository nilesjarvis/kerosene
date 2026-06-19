use crate::account::{
    AssetPosition, ClearinghouseState, OpenOrder, SpotBalance, UserFill, WalletPositionDetail,
};
use std::{collections::HashMap, fmt};

// ---------------------------------------------------------------------------
// User Stream Models
// ---------------------------------------------------------------------------

pub type KeyedUserData = (Option<String>, WsUserData);

#[derive(Clone)]
pub enum WsUserData {
    AllDexPositions {
        main_state: Box<ClearinghouseState>,
        states_by_dex: HashMap<String, ClearinghouseState>,
        all_positions: Vec<AssetPosition>,
        position_details: Vec<WalletPositionDetail>,
    },
    OpenOrders {
        dex: String,
        orders: Vec<OpenOrder>,
    },
    Fills {
        fills: Vec<UserFill>,
        is_snapshot: bool,
    },
    SpotBalances(Vec<SpotBalance>),
    AllMids(HashMap<String, f64>),
    /// The broadcast fanout for the user-data WebSocket signalled
    /// `RecvError::Lagged`: at least `skipped` order/fill/position
    /// updates were dropped before this consumer could observe them. The
    /// downstream handler must treat local account state as stale and
    /// force a full `fetch_account_data` rather than continuing from an
    /// unknown state.
    Lagged {
        skipped: u64,
    },
}

impl fmt::Debug for WsUserData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllDexPositions {
                main_state,
                states_by_dex,
                all_positions,
                position_details,
            } => f
                .debug_struct("AllDexPositions")
                .field(
                    "main_asset_positions",
                    &format_args!("len={}", main_state.asset_positions.len()),
                )
                .field(
                    "states_by_dex",
                    &format_args!("len={}", states_by_dex.len()),
                )
                .field(
                    "all_positions",
                    &format_args!("len={}", all_positions.len()),
                )
                .field(
                    "position_details",
                    &format_args!("len={}", position_details.len()),
                )
                .finish(),
            Self::OpenOrders { dex, orders } => f
                .debug_struct("OpenOrders")
                .field("dex", dex)
                .field("orders", &format_args!("len={}", orders.len()))
                .finish(),
            Self::Fills { fills, is_snapshot } => f
                .debug_struct("Fills")
                .field("fills", &format_args!("len={}", fills.len()))
                .field("is_snapshot", is_snapshot)
                .finish(),
            Self::SpotBalances(balances) => f
                .debug_tuple("SpotBalances")
                .field(&format_args!("len={}", balances.len()))
                .finish(),
            Self::AllMids(mids) => f
                .debug_tuple("AllMids")
                .field(&format_args!("len={}", mids.len()))
                .finish(),
            Self::Lagged { skipped } => f.debug_struct("Lagged").field("skipped", skipped).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::WalletPositionDetail;
    use serde_json::json;

    fn clearinghouse_state() -> ClearinghouseState {
        serde_json::from_value(json!({
            "marginSummary": {
                "accountValue": "100000.00",
                "totalNtlPos": "25000.00",
                "totalMarginUsed": "5000.00"
            },
            "crossMarginSummary": null,
            "crossMaintenanceMarginUsed": null,
            "withdrawable": "95000.00",
            "assetPositions": [{
                "position": {
                    "coin": "BTC",
                    "szi": "1.5",
                    "entryPx": "65000.00",
                    "positionValue": "97500.00",
                    "unrealizedPnl": "1234.56",
                    "liquidationPx": "42000.00",
                    "leverage": {
                        "type": "cross",
                        "value": 3
                    },
                    "marginUsed": "5000.00",
                    "cumFunding": null
                },
                "liquidationPx": "42000.00"
            }]
        }))
        .expect("clearinghouse state")
    }

    #[test]
    fn user_data_debug_summarizes_account_payloads() {
        let clearinghouse = clearinghouse_state();
        let position = clearinghouse.asset_positions[0].clone();
        let details = WalletPositionDetail {
            dex: "dex-a".to_string(),
            asset_position: position.clone(),
        };
        let mut states_by_dex = HashMap::new();
        states_by_dex.insert("dex-a".to_string(), clearinghouse.clone());

        let updates = [
            format!(
                "{:?}",
                WsUserData::AllDexPositions {
                    main_state: Box::new(clearinghouse),
                    states_by_dex,
                    all_positions: vec![position],
                    position_details: vec![details],
                }
            ),
            format!(
                "{:?}",
                WsUserData::OpenOrders {
                    dex: "dex-a".to_string(),
                    orders: vec![OpenOrder {
                        coin: "BTC".to_string(),
                        side: "B".to_string(),
                        limit_px: "65000.00".to_string(),
                        sz: "1.5".to_string(),
                        oid: 42,
                        timestamp: 1_000,
                        reduce_only: Some(false),
                        is_trigger: Some(false),
                        order_type: Some("Limit".to_string()),
                        tif: Some("Gtc".to_string()),
                        trigger_px: None,
                    }],
                }
            ),
            format!(
                "{:?}",
                WsUserData::Fills {
                    fills: vec![UserFill {
                        coin: "BTC".to_string(),
                        px: "65000.00".to_string(),
                        sz: "1.5".to_string(),
                        side: "B".to_string(),
                        time: 1_000,
                        hash: Some("fill-hash-sentinel".to_string()),
                        tid: Some(7),
                        oid: Some(42),
                        dir: "Open Long".to_string(),
                        closed_pnl: "1234.56".to_string(),
                        fee: "10.00".to_string(),
                    }],
                    is_snapshot: true,
                }
            ),
            format!(
                "{:?}",
                WsUserData::SpotBalances(vec![SpotBalance {
                    coin: "USDC".to_string(),
                    token: Some(0),
                    total: "95000.00".to_string(),
                    hold: "100.00".to_string(),
                    entry_ntl: "0.00".to_string(),
                    supplied: None,
                }])
            ),
            format!(
                "{:?}",
                WsUserData::AllMids(HashMap::from([("BTC".to_string(), 65000.0)]))
            ),
        ];

        for rendered in updates {
            assert!(rendered.contains("len=1"), "{rendered}");
            assert!(!rendered.contains("65000.00"), "{rendered}");
            assert!(!rendered.contains("95000.00"), "{rendered}");
            assert!(!rendered.contains("1234.56"), "{rendered}");
            assert!(!rendered.contains("fill-hash-sentinel"), "{rendered}");
        }
    }
}
