use crate::account::{
    AssetPosition, Position, PositionLeverage, SpotBalance, UserFill,
    derive_spot_cost_basis_from_fills,
};
use crate::api::{ExchangeSymbol, MarketType};
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
            positions.extend(
                data.spot.balances.iter().filter_map(|balance| {
                    self.spot_asset_position_for_balance(balance, &data.fills)
                }),
            );
        }

        positions
    }

    pub(crate) fn spot_asset_position_for_balance(
        &self,
        balance: &SpotBalance,
        fills: &[UserFill],
    ) -> Option<AssetPosition> {
        let trade_coins = self.spot_trade_coins_for_balance(balance)?;
        let trade_coin = self.select_spot_trade_coin(&trade_coins, balance, fills)?;
        let mark_px = self.resolve_mid_for_symbol(&trade_coin);
        spot_asset_position_from_balance(balance, trade_coin, mark_px, fills)
    }

    /// Live USD mark for a spot balance, resolved through the balance's spot
    /// trade pair (the same balance-to-pair mapping the positions table uses)
    /// rather than the bare token name, which is not a mids key.
    pub(crate) fn spot_balance_mark_price(
        &self,
        balance: &SpotBalance,
        fills: &[UserFill],
    ) -> Option<f64> {
        match self.spot_trade_coins_for_balance(balance) {
            Some(trade_coins) => {
                let trade_coin = self.select_spot_trade_coin(&trade_coins, balance, fills)?;
                self.resolve_mid_for_symbol(&trade_coin)
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

    fn select_spot_trade_coin(
        &self,
        trade_coins: &[String],
        balance: &SpotBalance,
        fills: &[UserFill],
    ) -> Option<String> {
        if trade_coins.len() <= 1 {
            return trade_coins.first().cloned();
        }
        // Duplicated tickers: prefer the market whose fills reconcile to the
        // live balance (the one the user actually traded), then the market
        // with the most recent fill — reconciliation fails transiently while
        // a trade settles because fills and balances stream independently,
        // and dropping straight to a mid-based pick would flip the row to a
        // different market mid-trade — then any market with a live mark, so
        // a stale or differently-quoted duplicate cannot misprice the
        // position.
        trade_coins
            .iter()
            .find(|trade_coin| {
                derive_spot_cost_basis_from_fills(balance, trade_coin, fills).is_some()
            })
            .or_else(|| {
                trade_coins
                    .iter()
                    .filter_map(|trade_coin| {
                        fills
                            .iter()
                            .filter(|fill| fill.coin == *trade_coin)
                            .map(|fill| fill.time)
                            .max()
                            .map(|last_fill_time| (last_fill_time, trade_coin))
                    })
                    .max_by_key(|(last_fill_time, _)| *last_fill_time)
                    .map(|(_, trade_coin)| trade_coin)
            })
            .or_else(|| {
                trade_coins
                    .iter()
                    .find(|trade_coin| self.resolve_mid_for_symbol(trade_coin).is_some())
            })
            .or_else(|| trade_coins.first())
            .cloned()
    }

    fn spot_trade_coins_for_balance(&self, balance: &SpotBalance) -> Option<Vec<String>> {
        if Self::outcome_balance_coin_to_trade_coin(&balance.coin).is_some()
            || spot_balance_is_stable(&balance.coin)
        {
            return None;
        }
        let total = parse_finite_number(&balance.total)?;
        if total.abs() <= POSITION_EPSILON {
            return None;
        }

        let spot_pairs: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| {
                symbol.market_type == MarketType::Spot
                    && symbol.ticker.eq_ignore_ascii_case(&balance.coin)
            })
            .collect();
        // Non-USD-quoted pairs report mids in quote units. Every consumer of
        // this mapping currently treats marks and position values as USD, so
        // fail closed when no USD-stable pair exists.
        let usd_quoted: Vec<&ExchangeSymbol> = spot_pairs
            .iter()
            .copied()
            .filter(|symbol| symbol.spot_quote_is_usd_stable())
            .collect();
        let mut trade_coins: Vec<(u32, String)> = usd_quoted
            .into_iter()
            .map(|symbol| (symbol.asset_index, symbol.key.clone()))
            .collect();
        if trade_coins.is_empty() {
            return None;
        }
        trade_coins.sort_by_key(|(asset_index, _)| *asset_index);
        Some(
            trade_coins
                .into_iter()
                .map(|(_, trade_coin)| trade_coin)
                .collect(),
        )
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

    fn spot_symbol_with_display(
        key: &str,
        ticker: &str,
        asset_index: u32,
        display: &str,
    ) -> ExchangeSymbol {
        ExchangeSymbol {
            display_name: Some(display.to_string()),
            ..spot_symbol(key, ticker, asset_index)
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

    fn account_data_with_fills(balances: Vec<SpotBalance>, fills: Vec<UserFill>) -> AccountData {
        let mut data = account_data(balances);
        data.fills = fills;
        data
    }

    fn spot_fill(
        coin: &str,
        px: &str,
        sz: &str,
        fee: &str,
        fee_token: &str,
        time: u64,
    ) -> UserFill {
        UserFill {
            coin: coin.to_string(),
            px: px.to_string(),
            sz: sz.to_string(),
            side: "B".to_string(),
            time,
            hash: None,
            tid: Some(time),
            oid: None,
            dir: "Buy".to_string(),
            closed_pnl: "0".to_string(),
            fee: fee.to_string(),
            fee_token: Some(fee_token.to_string()),
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
    fn spot_balances_use_reconciled_fill_cost_basis_when_entry_notional_is_missing() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("@142", "UBTC", 10_142)];
        set_mid(&mut terminal, "@142", 58_358.0);
        set_connected_account_data(
            &mut terminal,
            account_data_with_fills(
                vec![spot_balance("UBTC", "6.7491729032", "0.0")],
                vec![
                    spot_fill("@142", "60191", "1.0", "0.0004", "UBTC", 1),
                    spot_fill("@142", "58395", "5.753", "0.0034270968", "UBTC", 2),
                ],
            ),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@142");
        assert_wire_close(
            &positions[0].position.entry_px,
            (60_191.0 + 58_395.0 * 5.753) / 6.7491729032,
        );
        assert_wire_close(
            &positions[0].position.unrealized_pnl,
            6.7491729032 * 58_358.0 - (60_191.0 + 58_395.0 * 5.753),
        );
    }

    #[test]
    fn spot_balances_choose_fill_reconciled_candidate_when_ticker_is_duplicated() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            spot_symbol("@142", "UBTC", 10_142),
            spot_symbol("@234", "UBTC", 10_234),
        ];
        set_mid(&mut terminal, "@142", 58_000.0);
        set_mid(&mut terminal, "@234", 58_358.0);
        set_connected_account_data(
            &mut terminal,
            account_data_with_fills(
                vec![spot_balance("UBTC", "1", "0")],
                vec![spot_fill("@234", "60191", "1", "0", "USDC", 1)],
            ),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@234");
        assert_wire_close(&positions[0].position.entry_px, 60_191.0);
        assert_wire_close(&positions[0].position.unrealized_pnl, 58_358.0 - 60_191.0);
    }

    #[test]
    fn spot_balances_keep_traded_pair_when_fills_do_not_reconcile_mid_trade() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            spot_symbol("@142", "UBTC", 10_142),
            spot_symbol("@234", "UBTC", 10_234),
        ];
        set_mid(&mut terminal, "@142", 58_000.0);
        set_mid(&mut terminal, "@234", 58_358.0);
        // The buy fill has landed but the balance snapshot still reports the
        // pre-trade total, so no pair's fills reconcile. The row must stay on
        // the most recently traded pair instead of flipping to a duplicate
        // market, and must report no PnL rather than a possibly-wrong one.
        set_connected_account_data(
            &mut terminal,
            account_data_with_fills(
                vec![spot_balance("UBTC", "2", "0")],
                vec![spot_fill("@234", "60191", "1", "0", "USDC", 1)],
            ),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@234");
        assert_eq!(positions[0].position.entry_px, "");
        assert_eq!(positions[0].position.unrealized_pnl, "");
        assert_wire_close(&positions[0].position.position_value, 2.0 * 58_358.0);
    }

    #[test]
    fn spot_balances_with_entry_notional_choose_fill_reconciled_candidate_when_ticker_is_duplicated()
     {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            spot_symbol("@142", "UBTC", 10_142),
            spot_symbol("@234", "UBTC", 10_234),
        ];
        set_mid(&mut terminal, "@142", 58_000.0);
        set_mid(&mut terminal, "@234", 58_358.0);
        set_connected_account_data(
            &mut terminal,
            account_data_with_fills(
                vec![spot_balance("UBTC", "1", "60191")],
                vec![spot_fill("@234", "60191", "1", "0", "USDC", 1)],
            ),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@234");
        assert_wire_close(&positions[0].position.entry_px, 60_191.0);
        assert_wire_close(&positions[0].position.unrealized_pnl, 58_358.0 - 60_191.0);
    }

    #[test]
    fn spot_balances_with_entry_notional_prefer_live_mid_candidate_when_ticker_is_duplicated() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            spot_symbol("@142", "UBTC", 10_142),
            spot_symbol("@234", "UBTC", 10_234),
        ];
        set_mid(&mut terminal, "@234", 58_358.0);
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("UBTC", "1", "60191")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@234");
        assert_wire_close(&positions[0].position.position_value, 58_358.0);
        assert_wire_close(&positions[0].position.unrealized_pnl, 58_358.0 - 60_191.0);
    }

    #[test]
    fn spot_balances_ignore_non_usd_quoted_duplicates_for_valuation() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            spot_symbol("@142", "UETH", 10_142),
            spot_symbol_with_display("@151", "UETH", 10_151, "UETH/UBTC"),
        ];
        set_mid(&mut terminal, "@142", 2_500.0);
        set_mid(&mut terminal, "@151", 0.037);
        set_connected_account_data(
            &mut terminal,
            account_data_with_fills(
                vec![spot_balance("UETH", "1", "0")],
                // Fills reconcile to the UBTC-quoted pair, but its mid is in
                // UBTC units and must not be rendered as a USD value.
                vec![spot_fill("@151", "0.037", "1", "0", "USDC", 1)],
            ),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "@142");
        assert_wire_close(&positions[0].position.position_value, 2_500.0);
    }

    #[test]
    fn purr_balance_maps_to_api_named_spot_pair() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("PURR/USDC", "PURR", 10_000)];
        set_mid(&mut terminal, "PURR/USDC", 4.0);
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("PURR", "2", "3")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "PURR/USDC");
        assert_wire_close(&positions[0].position.position_value, 8.0);
        assert_wire_close(&positions[0].position.unrealized_pnl, 5.0);
    }

    #[test]
    fn spot_balance_mark_price_resolves_through_the_spot_pair() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("@77", "JEFF", 10_077)];
        set_mid(&mut terminal, "@77", 2.0);

        // The balance coin is the token name, which is not a mids key; the
        // mark must come from the "@N" spot pair instead.
        let balance = spot_balance("JEFF", "10", "5");
        assert_eq!(terminal.spot_balance_mark_price(&balance, &[]), Some(2.0));

        // Outcome balance coins keep their "+NNN" -> "#NNN" alias lookup.
        set_mid(&mut terminal, "#950", 0.6);
        let outcome_balance = spot_balance("+950", "30", "12");
        assert_eq!(
            terminal.spot_balance_mark_price(&outcome_balance, &[]),
            Some(0.6)
        );
    }

    #[test]
    fn outcome_balances_with_entry_notional_report_cost_basis_pnl() {
        let mut terminal = TradingTerminal::boot().0;
        set_mid(&mut terminal, "#950", 0.6);
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("+950", "30", "12")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "#950");
        assert_eq!(positions[0].position.leverage.leverage_type, "outcome");
        assert_wire_close(&positions[0].position.entry_px, 0.4);
        assert_wire_close(&positions[0].position.position_value, 18.0);
        assert_wire_close(&positions[0].position.unrealized_pnl, 6.0);
    }

    #[test]
    fn outcome_balances_without_live_mark_are_valued_at_cost_with_zero_pnl() {
        let mut terminal = TradingTerminal::boot().0;
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("+950", "30", "12")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "#950");
        assert_wire_close(&positions[0].position.entry_px, 0.4);
        assert_wire_close(&positions[0].position.position_value, 12.0);
        assert_wire_close(&positions[0].position.unrealized_pnl, 0.0);
    }

    #[test]
    fn outcome_balances_without_entry_notional_keep_pnl_unavailable() {
        let mut terminal = TradingTerminal::boot().0;
        set_mid(&mut terminal, "#950", 0.6);
        set_connected_account_data(
            &mut terminal,
            account_data(vec![spot_balance("+950", "30", "0")]),
        );

        let positions = terminal.account_positions_with_outcomes();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position.coin, "#950");
        assert_eq!(positions[0].position.entry_px, "");
        assert_wire_close(&positions[0].position.position_value, 18.0);
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
