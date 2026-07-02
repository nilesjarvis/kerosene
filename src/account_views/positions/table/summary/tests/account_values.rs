use super::*;
use crate::account::{
    AccountAbstractionMode, AccountData, ClearinghouseState, MarginSummary, SpotBalance,
    SpotClearinghouseState,
};
use crate::api::{ExchangeSymbol, MarketType};

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

fn account_data(
    abstraction: AccountAbstractionMode,
    perp_account_value: &str,
    balances: Vec<SpotBalance>,
) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: abstraction,
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: perp_account_value.to_string(),
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
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 1,
    }
}

fn spot_symbol(key: &str, ticker: &str, asset_index: u32) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(format!("{ticker}/USDC")),
        keywords: Vec::new(),
        asset_index,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn set_mid(terminal: &mut TradingTerminal, coin: &str, mid: f64) {
    terminal.all_mids.insert(coin.to_string(), mid);
    terminal
        .all_mids_updated_at_ms
        .insert(coin.to_string(), TradingTerminal::now_ms());
}

#[test]
fn spot_only_token_is_valued_at_its_spot_pair_mid() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.muted_tickers.clear();
    terminal.exchange_symbols = vec![spot_symbol("@77", "JEFF", 10_077)];
    set_mid(&mut terminal, "@77", 2.0);
    let data = account_data(
        AccountAbstractionMode::Disabled,
        "0",
        vec![spot_balance("JEFF", "10", "5")],
    );

    // The balance coin "JEFF" is not a mids key; the value must come from
    // the "@77" spot pair mid, not freeze at the entry notional.
    assert_eq!(terminal.position_summary_account_value(&data), Some(20.0));
}

#[test]
fn shared_balance_account_value_matches_header_dedupe_semantics() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.muted_tickers.clear();
    let data = account_data(
        AccountAbstractionMode::Default,
        "92",
        vec![spot_balance("USDC", "100", "0")],
    );

    // Shared-balance accounts mirror one USDC pool through both the perp
    // clearinghouse and spot balances; the PnL % denominator must dedupe it
    // exactly like the header total (max), not report ~192.
    assert_eq!(terminal.position_summary_account_value(&data), Some(100.0));
}

#[test]
fn non_shared_account_value_still_sums_perp_equity_and_spot() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.muted_tickers.clear();
    let data = account_data(
        AccountAbstractionMode::Disabled,
        "92",
        vec![spot_balance("USDC", "100", "0")],
    );

    assert_eq!(terminal.position_summary_account_value(&data), Some(192.0));
}
