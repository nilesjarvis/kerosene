use crate::account::{
    AssetPosition, Position, PositionLeverage, SpotBalance, UserFill,
    derive_spot_cost_basis_from_fills,
};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;
use crate::signing::float_to_wire;

use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Account Position Projection
// ---------------------------------------------------------------------------

const POSITION_EPSILON: f64 = 1e-12;

impl TradingTerminal {
    /// Borrow native positions and eagerly append owned outcome and spot rows.
    pub(crate) fn account_positions_with_outcomes(&self) -> Vec<Cow<'_, AssetPosition>> {
        let mut positions = Vec::new();
        let Some((_, data)) = self.connected_order_account_snapshot() else {
            return positions;
        };

        positions.extend(data.clearinghouse.asset_positions.iter().map(Cow::Borrowed));
        // Synthesize a position for every outcome balance coin, even when the
        // market is expired, still loading, or a fallback-settlement contract:
        // a held balance is a real position and must not vanish from the
        // Positions tab just because the symbol lookup misses.
        positions.extend(data.spot.balances.iter().filter_map(|balance| {
            let trade_coin = Self::outcome_balance_coin_to_trade_coin(&balance.coin)?;
            let mark_px = self.resolve_mid_for_symbol(&trade_coin);
            outcome_asset_position_from_balance(balance, trade_coin, mark_px).map(Cow::Owned)
        }));
        if self.account_view_includes_spot_balances(data) {
            positions.extend(data.spot.balances.iter().filter_map(|balance| {
                self.spot_asset_position_for_balance(balance, &data.fills)
                    .map(Cow::Owned)
            }));
        }

        positions
    }

    pub(crate) fn spot_asset_position_for_balance(
        &self,
        balance: &SpotBalance,
        fills: &[UserFill],
    ) -> Option<AssetPosition> {
        let pairs = self.spot_pairs_for_balance(balance)?;
        let pair = self.select_spot_pair(&pairs, balance, fills)?;
        let mark_px = self.resolve_mid_for_symbol(&pair.key);
        spot_asset_position_from_balance(balance, pair.key.clone(), mark_px, fills)
    }

    /// Live USD mark for a spot balance, resolved through the balance's spot
    /// trade pair (the same balance-to-pair mapping the positions table uses)
    /// rather than the bare token name, which is not a mids key.
    pub(crate) fn spot_balance_mark_price(
        &self,
        balance: &SpotBalance,
        fills: &[UserFill],
    ) -> Option<f64> {
        match self.spot_pairs_for_balance(balance) {
            Some(pairs) => {
                let pair = self.select_spot_pair(&pairs, balance, fills)?;
                self.resolve_mid_for_symbol(&pair.key)
            }
            // Stables and outcome balance coins keep the direct lookup
            // ("+NNN" outcome coins resolve via their "#" alias). Never use a
            // same-ticker perp or a crypto-quoted pair as a USD spot mark.
            None if spot_balance_is_stable(&balance.coin)
                || Self::outcome_balance_coin_to_trade_coin(&balance.coin).is_some() =>
            {
                self.resolve_mid_for_symbol(&balance.coin)
            }
            None => None,
        }
    }

    fn select_spot_pair<'a>(
        &self,
        pairs: &[&'a ExchangeSymbol],
        balance: &SpotBalance,
        fills: &[UserFill],
    ) -> Option<&'a ExchangeSymbol> {
        if pairs.len() <= 1 {
            return pairs.first().copied();
        }
        // Duplicated tickers: prefer the market whose fills reconcile to the
        // live balance (the one the user actually traded), then the market
        // with the most recent fill — reconciliation fails transiently while
        // a trade settles because fills and balances stream independently,
        // and dropping straight to a mid-based pick would flip the row to a
        // different market mid-trade — then any market with a live mark, so
        // a stale or differently-quoted duplicate cannot misprice the
        // position.
        pairs
            .iter()
            .copied()
            .find(|pair| derive_spot_cost_basis_from_fills(balance, &pair.key, fills).is_some())
            .or_else(|| {
                pairs
                    .iter()
                    .copied()
                    .filter_map(|pair| {
                        fills
                            .iter()
                            .filter(|fill| fill.coin == pair.key)
                            .map(|fill| fill.time)
                            .max()
                            .map(|last_fill_time| (last_fill_time, pair))
                    })
                    .max_by_key(|(last_fill_time, _)| *last_fill_time)
                    .map(|(_, pair)| pair)
            })
            .or_else(|| {
                pairs
                    .iter()
                    .copied()
                    .find(|pair| self.resolve_mid_for_symbol(&pair.key).is_some())
            })
            .or_else(|| pairs.first().copied())
    }

    fn spot_pairs_for_balance(&self, balance: &SpotBalance) -> Option<Vec<&ExchangeSymbol>> {
        if Self::outcome_balance_coin_to_trade_coin(&balance.coin).is_some()
            || spot_balance_is_stable(&balance.coin)
        {
            return None;
        }
        let total = parse_finite_number(&balance.total)?;
        if total.abs() <= POSITION_EPSILON {
            return None;
        }

        // Non-USD-quoted pairs report mids in quote units. Every consumer of
        // this mapping currently treats marks and position values as USD, so
        // fail closed when no USD-stable pair exists.
        let mut pairs: Vec<_> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| {
                symbol.market_type == MarketType::Spot
                    && symbol.ticker.eq_ignore_ascii_case(&balance.coin)
            })
            .filter(|symbol| symbol.spot_quote_is_usd_stable())
            .collect();
        if pairs.is_empty() {
            return None;
        }
        pairs.sort_by_key(|symbol| symbol.asset_index);
        Some(pairs)
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
    let entry_notional = spot_balance_entry_notional(balance);
    let entry_px = entry_notional.map(|entry_notional| entry_notional / size);
    // Expired or settling outcome markets have no live mark; fall back to
    // valuing the balance at cost so it stays visible with zero PnL.
    let position_value = mark_px.map(|mark_px| size * mark_px).or(entry_notional);
    // Without an entry notional (e.g. a transferred-in balance) PnL is
    // unavailable — it must not report the full position value as profit.
    let unrealized_pnl = entry_notional
        .zip(position_value)
        .map(|(entry_notional, position_value)| position_value - entry_notional);

    Some(AssetPosition {
        position: Position {
            coin: trade_coin,
            szi: float_to_wire(total),
            entry_px: entry_px.map(float_to_wire).unwrap_or_default(),
            position_value: position_value.map(float_to_wire).unwrap_or_default(),
            unrealized_pnl: unrealized_pnl.map(float_to_wire).unwrap_or_default(),
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
    fills: &[UserFill],
) -> Option<AssetPosition> {
    let total = parse_finite_number(&balance.total)?;
    if total.abs() <= POSITION_EPSILON {
        return None;
    }

    let size = total.abs();
    let entry_notional = spot_balance_entry_notional(balance).or_else(|| {
        derive_spot_cost_basis_from_fills(balance, &trade_coin, fills)
            .map(|basis| basis.entry_notional)
    });
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

fn spot_balance_entry_notional(balance: &SpotBalance) -> Option<f64> {
    parse_finite_number(&balance.entry_ntl)
        .filter(|entry_notional| entry_notional.abs() > POSITION_EPSILON)
        .map(f64::abs)
}

fn spot_balance_is_stable(coin: &str) -> bool {
    matches!(coin, "USDC" | "USDE" | "USDT0" | "USDH")
}

#[cfg(test)]
mod tests;
