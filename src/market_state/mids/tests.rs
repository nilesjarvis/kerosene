use super::*;
use crate::config::{DisplayDenominationConfig, MarketUniverseConfig};

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
