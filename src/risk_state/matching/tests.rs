use super::*;
use crate::account::{
    AccountAbstractionMode, AccountData, ClearinghouseState, MarginSummary, SpotClearinghouseState,
};

fn symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "test".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn perp_symbol_with_collateral(key: &str, token: Option<u32>) -> ExchangeSymbol {
    ExchangeSymbol {
        collateral_token: token,
        ..symbol(key, key.rsplit(':').next().unwrap_or(key), MarketType::Perp)
    }
}

fn account_data_with_mode(
    abstraction: AccountAbstractionMode,
    portfolio_margin_enabled: bool,
) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: abstraction,
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
            balances: Vec::new(),
            portfolio_margin_enabled,
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

#[test]
fn hip3_market_universe_matches_only_selected_perp_dex() {
    let universe = MarketUniverseConfig::hip3_dex("xyz");
    let xyz_nvda = symbol("xyz:NVDA", "NVDA", MarketType::Perp);
    let flx_nvda = symbol("flx:NVDA", "NVDA", MarketType::Perp);
    let spot = symbol("@107", "HYPE", MarketType::Spot);

    assert!(TradingTerminal::symbol_matches_market_universe(
        &xyz_nvda, &universe
    ));
    assert!(!TradingTerminal::symbol_matches_market_universe(
        &flx_nvda, &universe
    ));
    assert!(!TradingTerminal::symbol_matches_market_universe(
        &spot, &universe
    ));
}

#[test]
fn hip3_market_universe_matches_raw_dex_prefixed_keys_without_symbol_metadata() {
    let symbols = Vec::new();
    let universe = MarketUniverseConfig::hip3_dex("xyz");

    assert!(TradingTerminal::key_matches_market_universe(
        &symbols, &universe, "xyz:NVDA"
    ));
    assert!(!TradingTerminal::key_matches_market_universe(
        &symbols, &universe, "NVDA"
    ));
    assert!(!TradingTerminal::key_matches_market_universe(
        &symbols, &universe, "flx:NVDA"
    ));
}

#[test]
fn market_universe_options_include_discovered_dexes_not_only_known_constants() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![perp_symbol_with_collateral("newdex:ABC", Some(404))];

    assert!(
        terminal
            .market_universe_options()
            .contains(&MarketUniverseConfig::hip3_dex("newdex"))
    );
}

#[test]
fn selected_hip3_collateral_token_requires_symbol_metadata() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("newdex");
    terminal.exchange_symbols = vec![perp_symbol_with_collateral("newdex:ABC", Some(404))];

    assert_eq!(terminal.visible_collateral_token(), Some(404));

    terminal.exchange_symbols.clear();
    assert_eq!(terminal.visible_collateral_token(), None);
}

#[test]
fn portfolio_margin_keeps_spot_balances_visible_in_selected_hip3_universe() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    let data = account_data_with_mode(AccountAbstractionMode::PortfolioMargin, true);

    assert!(terminal.account_view_includes_spot_balances(&data));
    assert!(!terminal.account_spot_balance_is_hidden(&data, "USDC"));

    terminal.muted_tickers.insert("USDC".to_string());
    assert!(terminal.account_spot_balance_is_hidden(&data, "USDC"));
}

#[test]
fn non_portfolio_selected_hip3_still_hides_spot_balances() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
    let data = account_data_with_mode(AccountAbstractionMode::DexAbstraction, false);

    assert!(!terminal.account_view_includes_spot_balances(&data));
    assert!(terminal.account_spot_balance_is_hidden(&data, "USDC"));
}
