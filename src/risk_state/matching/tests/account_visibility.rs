use super::*;

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

#[test]
fn fallback_outcome_balances_are_hidden_even_when_all_markets_are_visible() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![outcome_symbol("#660", true), outcome_symbol("#670", false)];
    let data = account_data_with_mode(AccountAbstractionMode::UnifiedAccount, false);

    assert!(terminal.account_spot_balance_is_hidden(&data, "+660"));
    assert!(!terminal.account_spot_balance_is_hidden(&data, "+670"));
}
