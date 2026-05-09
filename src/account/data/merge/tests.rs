use crate::account::{
    AssetPosition, ClearinghouseState, MarginSummary, OpenOrder, Position, PositionLeverage,
};

use super::{merge_hip3_open_orders, merge_hip3_positions};

fn margin_summary() -> MarginSummary {
    MarginSummary {
        account_value: "1000".to_string(),
        total_ntl_pos: "0".to_string(),
        total_margin_used: "0".to_string(),
    }
}

fn clearinghouse(coins: &[&str]) -> ClearinghouseState {
    ClearinghouseState {
        margin_summary: margin_summary(),
        cross_margin_summary: None,
        cross_maintenance_margin_used: None,
        withdrawable: "1000".to_string(),
        asset_positions: coins
            .iter()
            .map(|coin| AssetPosition {
                position: Position {
                    coin: (*coin).to_string(),
                    szi: "1".to_string(),
                    entry_px: "10".to_string(),
                    position_value: "10".to_string(),
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
            })
            .collect(),
    }
}

fn open_order(coin: &str, oid: u64) -> OpenOrder {
    OpenOrder {
        coin: coin.to_string(),
        side: "B".to_string(),
        limit_px: "10".to_string(),
        sz: "1".to_string(),
        oid,
        timestamp: oid,
        reduce_only: Some(false),
    }
}

#[test]
fn hip3_positions_are_appended_to_main_clearinghouse() {
    let merged = merge_hip3_positions(
        clearinghouse(&["BTC"]),
        vec![clearinghouse(&["ETH"]), clearinghouse(&["SOL", "HYPE"])],
    );

    assert_eq!(
        merged
            .asset_positions
            .iter()
            .map(|position| position.position.coin.as_str())
            .collect::<Vec<_>>(),
        vec!["BTC", "ETH", "SOL", "HYPE"]
    );
}

#[test]
fn hip3_open_orders_are_appended_to_main_orders() {
    let merged = merge_hip3_open_orders(
        vec![open_order("BTC", 1)],
        vec![
            vec![open_order("ETH", 2)],
            vec![open_order("SOL", 3), open_order("HYPE", 4)],
        ],
    );

    assert_eq!(
        merged
            .iter()
            .map(|order| (order.coin.as_str(), order.oid))
            .collect::<Vec<_>>(),
        vec![("BTC", 1), ("ETH", 2), ("SOL", 3), ("HYPE", 4)]
    );
}
