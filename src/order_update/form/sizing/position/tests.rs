use super::*;
use crate::account::{
    AssetPosition, ClearinghouseState, MarginSummary, Position, PositionLeverage,
};

fn asset_position(coin: &str, szi: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
            entry_px: "100".to_string(),
            position_value: "0".to_string(),
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

fn clearinghouse(positions: Vec<AssetPosition>) -> ClearinghouseState {
    ClearinghouseState {
        margin_summary: MarginSummary {
            account_value: "0".to_string(),
            total_ntl_pos: "0".to_string(),
            total_margin_used: "0".to_string(),
        },
        cross_margin_summary: None,
        cross_maintenance_margin_used: None,
        withdrawable: "0".to_string(),
        asset_positions: positions,
    }
}

#[test]
fn position_quantity_percentage_handles_coin_and_usd_quantities() {
    assert_eq!(
        percentage_for_position_quantity(0.5, 2.0, false, None),
        25.0
    );
    assert_eq!(
        percentage_for_position_quantity(50.0, 2.0, true, Some(100.0)),
        25.0
    );
    assert_eq!(percentage_for_position_quantity(10.0, 2.0, true, None), 0.0);
}

#[test]
fn position_quantity_for_percentage_formats_coin_or_usd_quantity() {
    assert_eq!(
        position_quantity_for_percentage(50.0, 2.0, false, None, 5),
        "1.00000"
    );
    assert_eq!(
        position_quantity_for_percentage(25.0, 2.0, true, Some(100.0), 5),
        "50.00"
    );
    assert_eq!(
        position_quantity_for_percentage(f32::NAN, 2.0, false, None, 5),
        "0"
    );
}

#[test]
fn position_lookup_prefers_exact_match_then_prefixed_active_symbol_suffix() {
    let clearinghouse = clearinghouse(vec![
        asset_position("BTC", "1"),
        asset_position("xyz:BTC", "3"),
        asset_position("ETH", "-2"),
    ]);

    assert_eq!(position_size_for_symbol(&clearinghouse, "BTC"), Some(1.0));
    assert_eq!(
        position_size_for_symbol(&clearinghouse, "xyz:BTC"),
        Some(3.0)
    );
    assert_eq!(
        position_size_for_symbol(&clearinghouse, "abc:ETH"),
        Some(2.0)
    );
}

#[test]
fn position_lookup_rejects_invalid_zero_or_missing_sizes() {
    let clearinghouse = clearinghouse(vec![
        asset_position("BTC", "0"),
        asset_position("ETH", "bad"),
    ]);

    assert_eq!(position_size_for_symbol(&clearinghouse, "BTC"), None);
    assert_eq!(position_size_for_symbol(&clearinghouse, "ETH"), None);
    assert_eq!(position_size_for_symbol(&clearinghouse, "SOL"), None);
}
