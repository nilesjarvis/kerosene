use super::*;
use crate::config::{DisplayDenominationConfig, MarketUniverseConfig};

#[test]
fn all_markets_fetch_scope_includes_registered_dexes_without_active_symbols() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::All;
    terminal.exchange_symbols.clear();
    terminal.perp_dexes = vec![crate::api::PerpDex {
        name: "newdex".to_string(),
        collateral_token: Some(0),
    }];
    assert!(
        terminal
            .visible_mids_dexes()
            .iter()
            .any(|dex| dex == "newdex")
    );
    assert!(
        terminal
            .account_data_fetch_scope()
            .hip3_dexes(&[])
            .iter()
            .any(|dex| dex == "newdex")
    );
}

#[test]
fn visible_mids_dexes_include_display_denomination_dex() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("flx");
    terminal.display_denomination = DisplayDenominationConfig::eur();

    assert_eq!(
        terminal.visible_mids_dexes(),
        vec!["flx".to_string(), "xyz".to_string()]
    );
}

#[test]
fn visible_mids_dexes_include_main_dex_for_hype_display_denomination() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("flx");
    terminal.display_denomination = DisplayDenominationConfig::hype();

    assert_eq!(
        terminal.visible_mids_dexes(),
        vec![String::new(), "flx".to_string()]
    );
}

#[test]
fn visible_mids_dexes_include_main_dex_for_btc_display_denomination() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.market_universe = MarketUniverseConfig::hip3_dex("flx");
    terminal.display_denomination = DisplayDenominationConfig::btc();

    assert_eq!(
        terminal.visible_mids_dexes(),
        vec![String::new(), "flx".to_string()]
    );
}
