use super::*;

#[test]
fn selected_hip3_collateral_token_requires_symbol_metadata() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("newdex");
    terminal.exchange_symbols = vec![perp_symbol_with_collateral("newdex:ABC", Some(404))];

    assert_eq!(terminal.visible_collateral_token(), Some(404));

    terminal.exchange_symbols.clear();
    assert_eq!(terminal.visible_collateral_token(), None);
}
