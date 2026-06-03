use super::reconcile_current_position_trades;
use crate::account::{AssetPosition, Position, PositionLeverage};
use crate::journal::AggregatedTrade;

fn asset_position(coin: &str, szi: &str, entry_px: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
            entry_px: entry_px.to_string(),
            position_value: "1000".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "0".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

fn open_trade(coin: &str) -> AggregatedTrade {
    AggregatedTrade {
        id: format!("perp:{coin}:fill"),
        legacy_note_ids: Vec::new(),
        coin: coin.to_string(),
        start_time: 1_000,
        end_time: None,
        max_position: 2.0,
        volume: 200.0,
        fee: 1.0,
        pnl: 0.0,
        status: "OPEN".to_string(),
        fill_count: 2,
        avg_entry_price: 100.0,
        total_entry_notional: 200.0,
        total_entry_size: 2.0,
        is_long: true,
        basis_complete: true,
    }
}

#[test]
fn current_position_without_fills_adds_partial_open_trade() {
    let mut trades = Vec::new();

    let result = reconcile_current_position_trades(
        &mut trades,
        &[asset_position("ZEC", "-57459.53", "626.4693")],
        12_345,
    );

    assert_eq!(result.added_open_positions, 1);
    assert_eq!(trades.len(), 1);
    let trade = &trades[0];
    assert_eq!(trade.id, "position:ZEC");
    assert_eq!(trade.coin, "ZEC");
    assert_eq!(trade.status, "OPEN");
    assert!(!trade.basis_complete);
    assert!(!trade.is_long);
    assert_eq!(trade.start_time, 12_345);
    assert_eq!(trade.fill_count, 0);
    assert_eq!(trade.max_position, -57459.53);
    assert_eq!(trade.avg_entry_price, 626.4693);
}

#[test]
fn current_position_does_not_duplicate_fill_derived_open_trade() {
    let mut trades = vec![open_trade("ZEC")];

    let result = reconcile_current_position_trades(
        &mut trades,
        &[asset_position("ZEC", "-57459.53", "626.4693")],
        12_345,
    );

    assert_eq!(result.added_open_positions, 0);
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].id, "perp:ZEC:fill");
}

#[test]
fn current_position_reconciliation_removes_stale_synthetic_trades() {
    let mut trades = Vec::new();
    reconcile_current_position_trades(&mut trades, &[asset_position("ZEC", "-1", "600")], 1_000);

    let result = reconcile_current_position_trades(&mut trades, &[], 2_000);

    assert_eq!(result.removed_stale_positions, 1);
    assert!(trades.is_empty());
}
