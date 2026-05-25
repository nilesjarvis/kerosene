use super::*;

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
