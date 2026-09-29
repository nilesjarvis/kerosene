use super::*;
use crate::account::{
    AccountData, AccountDataCompleteness, ClearinghouseState, MarginSummary,
    SpotClearinghouseState, UserFeeRates,
};
use crate::api::ExchangeSymbol;

mod projection;
mod selection;

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
        growth_mode: false,
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

fn spot_fill(coin: &str, px: &str, sz: &str, fee: &str, fee_token: &str, time: u64) -> UserFill {
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
        start_position: None,
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
fn spot_balances_with_entry_notional_choose_fill_reconciled_candidate_when_ticker_is_duplicated() {
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
