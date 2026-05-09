use super::*;
use crate::account::{AssetPosition, CumFunding, Position, PositionLeverage};

fn asset_position(liquidation_px: Option<&str>, wrapper_liq: Option<&str>) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: "BTC".to_string(),
            szi: "1".to_string(),
            entry_px: "100".to_string(),
            position_value: "100".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: liquidation_px.map(str::to_string),
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "10".to_string(),
            cum_funding: None,
        },
        liquidation_px: wrapper_liq.map(str::to_string),
    }
}

#[test]
fn liquidation_price_parser_rejects_nonpositive_or_nonfinite_values() {
    assert_eq!(
        TradingTerminal::parse_liquidation_px(&asset_position(Some("50"), None)),
        Some(50.0)
    );
    assert_eq!(
        TradingTerminal::parse_liquidation_px(&asset_position(Some("inf"), None)),
        None
    );
    assert_eq!(
        TradingTerminal::parse_liquidation_px(&asset_position(Some("0"), None)),
        None
    );
}

#[test]
fn funding_pnl_rejects_nonfinite_values() {
    assert_eq!(
        TradingTerminal::position_funding_pnl(Some(&CumFunding {
            since_open: "-2.5".to_string()
        })),
        Some(2.5)
    );
    assert_eq!(
        TradingTerminal::position_funding_pnl(Some(&CumFunding {
            since_open: "NaN".to_string()
        })),
        None
    );
    assert_eq!(
        TradingTerminal::position_funding_pnl(Some(&CumFunding {
            since_open: "inf".to_string()
        })),
        None
    );
}
