use crate::account::{AssetPosition, Position, PositionLeverage, SpotBalance};
use crate::api::MarketType;
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
        let Some((_, data)) = self.connected_order_account_snapshot() else {
            return positions;
        };

        positions.extend(data.clearinghouse.asset_positions.iter().cloned());
        // Synthesize a position for every outcome balance coin, even when the
        // market is expired, still loading, or a fallback-settlement contract:
        // a held balance is a real position and must not vanish from the
        // Positions tab just because the symbol lookup misses.
        positions.extend(data.spot.balances.iter().filter_map(|balance| {
            let trade_coin = Self::outcome_balance_coin_to_trade_coin(&balance.coin)?;
            let mark_px = self.resolve_mid_for_symbol(&trade_coin);
            outcome_asset_position_from_balance(balance, trade_coin, mark_px)
        }));
        if self.account_view_includes_spot_balances(data) {
            positions.extend(data.spot.balances.iter().filter_map(|balance| {
                let trade_coin = self.spot_trade_coin_for_balance(balance)?;
                let mark_px = self.resolve_mid_for_symbol(&trade_coin);
                spot_asset_position_from_balance(balance, trade_coin, mark_px)
            }));
        }

        positions
    }

    fn spot_trade_coin_for_balance(&self, balance: &SpotBalance) -> Option<String> {
        if Self::outcome_balance_coin_to_trade_coin(&balance.coin).is_some()
            || spot_balance_is_stable(&balance.coin)
        {
            return None;
        }
        let total = parse_finite_number(&balance.total)?;
        if total.abs() <= POSITION_EPSILON {
            return None;
        }

        self.exchange_symbols
            .iter()
            .filter(|symbol| {
                symbol.market_type == MarketType::Spot
                    && symbol.ticker.eq_ignore_ascii_case(&balance.coin)
            })
            .min_by_key(|symbol| symbol.asset_index)
            .map(|symbol| symbol.key.clone())
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

fn spot_asset_position_from_balance(
    balance: &SpotBalance,
    trade_coin: String,
    mark_px: Option<f64>,
) -> Option<AssetPosition> {
    let total = parse_finite_number(&balance.total)?;
    if total.abs() <= POSITION_EPSILON {
        return None;
    }

    let size = total.abs();
    let entry_notional = parse_finite_number(&balance.entry_ntl)
        .filter(|entry_notional| entry_notional.abs() > POSITION_EPSILON)
        .map(f64::abs);
    let entry_px = entry_notional.map(|entry_notional| entry_notional / size);
    let position_value = mark_px
        .map(|mark_px| size * mark_px)
        .or(entry_notional)
        .map(float_to_wire)
        .unwrap_or_default();
    let unrealized_pnl = entry_notional
        .zip(mark_px)
        .map(|(entry_notional, mark_px)| size * mark_px - entry_notional)
        .map(float_to_wire)
        .unwrap_or_default();

    Some(AssetPosition {
        position: Position {
            coin: trade_coin,
            szi: float_to_wire(total),
            entry_px: entry_px.map(float_to_wire).unwrap_or_default(),
            position_value,
            unrealized_pnl,
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "spot".to_string(),
                value: 1,
            },
            margin_used: String::new(),
            cum_funding: None,
        },
        liquidation_px: None,
    })
}

fn spot_balance_is_stable(coin: &str) -> bool {
    matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
        SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::ExchangeSymbol;

    const TEST_ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";

    fn spot_symbol(key: &str, ticker: &str, asset_index: u32) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: ticker.to_string(),
            category: "spot".to_string(),
            display_name: Some(format!("{ticker}/USDC")),
            keywords: Vec::new(),
            asset_index,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Spot,
            outcome: None,
        }
    }

    fn spot_balance(coin: &str, total: &str, entry_ntl: &str) -> SpotBalance {
        SpotBalance {
            coin: coin.to_string(),
            token: None,
            total: total.to_string(),
            hold: "0".to_string(),
            entry_ntl: entry_ntl.to_string(),
            supplied: None,
        }
    }

    fn account_data(balances: Vec<SpotBalance>) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: Vec::new(),
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances,
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders: Vec::new(),
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1_000,
        }
    }

    fn set_connected_account_data(terminal: &mut TradingTerminal, data: AccountData) {
        terminal.connected_address = Some(TEST_ACCOUNT.to_string());
        terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
    }

    fn set_mid(terminal: &mut TradingTerminal, coin: &str, mid: f64) {
        terminal.all_mids.insert(coin.to_string(), mid);
        terminal
            .all_mids_updated_at_ms
            .insert(coin.to_string(), TradingTerminal::now_ms());
    }

    fn assert_wire_close(raw: &str, expected: f64) {
        let actual = raw
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("expected numeric wire value, got {raw:?}"));
        let tolerance = expected.abs().max(1.0) * 1e-10;
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn regular_spot_balances_are_projected_into_positions_with_pnl() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", 10_107)];
        set_mid(&mut terminal, "@107", 64.553);
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("HYPE", "0.00310065", "0.22353487")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@107");
        assert_eq!(positions[0].position.leverage.leverage_type, "spot");
        assert_wire_close(&positions[0].position.entry_px, 0.22353487 / 0.00310065);
        assert_wire_close(&positions[0].position.position_value, 0.00310065 * 64.553);
        assert_wire_close(
            &positions[0].position.unrealized_pnl,
            0.00310065 * 64.553 - 0.22353487,
        );
    }

    #[test]
    fn spot_balances_without_entry_notional_keep_pnl_unavailable() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("@142", "UBTC", 10_142)];
        set_mid(&mut terminal, "@142", 58_358.0);
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("UBTC", "6.7491729032", "0.0")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@142");
        assert_eq!(positions[0].position.entry_px, "");
        assert_wire_close(
            &positions[0].position.position_value,
            6.7491729032 * 58_358.0,
        );
        assert_eq!(positions[0].position.unrealized_pnl, "");
    }

    #[test]
    fn stablecoin_balances_are_not_projected_as_positions() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", 10_107)];
        set_connected_account_data(
            &mut terminal,
            account_data(vec![
                spot_balance("USDC", "100", "0"),
                spot_balance("HYPE", "0", "0"),
            ]),
        );

        assert!(terminal.account_positions_with_outcomes().is_empty());
    }
}
