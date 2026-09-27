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

#[test]
fn market_universe_picker_includes_and_labels_registered_dexes_without_active_markets() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.perp_dexes = ["active", "inactive", "empty"]
        .into_iter()
        .map(|name| crate::api::PerpDex {
            name: name.to_string(),
            collateral_token: Some(0),
        })
        .collect();
    terminal.exchange_symbols = vec![perp_symbol_with_collateral("active:ABC", Some(0))];
    terminal.market_universe = MarketUniverseConfig::hip3_dex("inactive");

    let (options, selected) = terminal.market_universe_picker_options();
    assert_eq!(
        options.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec![
            "All Markets",
            "HIP-3: active",
            "HIP-3: empty (no active markets)",
            "HIP-3: inactive (no active markets)"
        ]
    );
    assert!(options.contains(&selected));
    assert_eq!(selected.universe, terminal.market_universe);
    assert_eq!(
        terminal.normalize_market_universe_selection(terminal.market_universe.clone()),
        terminal.market_universe
    );
    assert_eq!(
        terminal.normalize_market_universe_selection(MarketUniverseConfig::hip3_dex("unknown")),
        MarketUniverseConfig::All
    );
}

#[test]
fn selecting_registered_inactive_dex_scopes_account_without_selecting_a_contract() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::All;
    terminal.exchange_symbols = vec![perp_symbol_with_collateral("BTC", Some(0))];
    terminal.perp_dexes = vec![crate::api::PerpDex {
        name: "inactive".to_string(),
        collateral_token: Some(404),
    }];
    terminal.active_symbol = "BTC".to_string();

    let _task = terminal.update(crate::message::Message::MarketUniverseChanged(
        MarketUniverseConfig::hip3_dex("inactive"),
    ));

    assert_eq!(
        terminal.market_universe.selected_hip3_dex(),
        Some("inactive")
    );
    assert_eq!(
        terminal.account_data_fetch_scope().selected_hip3_dex(),
        Some("inactive")
    );
    assert!(terminal.active_symbol.is_empty());
    assert!(terminal.fallback_unmuted_symbol_key().is_none());
    assert_eq!(terminal.visible_collateral_token(), Some(404));
}
