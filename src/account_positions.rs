use crate::account::{AssetPosition, Position, PositionLeverage, SpotBalance};
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;
use crate::signing::float_to_wire;

// ---------------------------------------------------------------------------
// Account Position Projection
// ---------------------------------------------------------------------------

const POSITION_EPSILON: f64 = 1e-12;

impl TradingTerminal {
    pub(crate) fn account_positions_with_outcomes(&self) -> Vec<AssetPosition> {
        let mut positions = Vec::new();
        let Some(data) = self.account_data.as_ref() else {
            return positions;
        };

        positions.extend(data.clearinghouse.asset_positions.iter().cloned());
        positions.extend(data.spot.balances.iter().filter_map(|balance| {
            let trade_coin = self.outcome_trade_coin_for_balance_coin(&balance.coin)?;
            let mark_px = self.resolve_mid_for_symbol(&trade_coin);
            outcome_asset_position_from_balance(balance, trade_coin, mark_px)
        }));

        positions
    }
}

fn outcome_asset_position_from_balance(
    balance: &SpotBalance,
    trade_coin: String,
    mark_px: Option<f64>,
) -> Option<AssetPosition> {
    let total = parse_finite_number(&balance.total)?;
    if total.abs() <= POSITION_EPSILON {
        return None;
    }

    let size = total.abs();
    let entry_notional = parse_finite_number(&balance.entry_ntl).unwrap_or(0.0).abs();
    let entry_px = if entry_notional > POSITION_EPSILON {
        entry_notional / size
    } else {
        mark_px.unwrap_or(0.0)
    };
    let position_value = mark_px
        .map(|mark_px| size * mark_px)
        .or_else(|| (entry_notional > POSITION_EPSILON).then_some(entry_notional))
        .unwrap_or(0.0);
    let unrealized_pnl = position_value - entry_notional;

    Some(AssetPosition {
        position: Position {
            coin: trade_coin,
            szi: float_to_wire(total),
            entry_px: float_to_wire(entry_px),
            position_value: float_to_wire(position_value),
            unrealized_pnl: float_to_wire(unrealized_pnl),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "outcome".to_string(),
                value: 1,
            },
            margin_used: String::new(),
            cum_funding: None,
        },
        liquidation_px: None,
    })
}
