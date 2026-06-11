use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;

fn spot_symbol(key: &str, display_name: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "spot".to_string(),
        display_name: Some(display_name.to_string()),
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

#[test]
fn wallet_order_symbol_label_resolves_outcome_and_spot_coins() {
    let mut terminal = TradingTerminal::boot().0;
    terminal
        .outcome_display_labels
        .insert("#950".to_string(), "YES: Will BTC close green?".to_string());
    terminal
        .exchange_symbols
        .push(spot_symbol("@107", "HYPE/USDC"));

    assert_eq!(
        terminal.wallet_order_symbol_label("", "#950"),
        "YES: Will BTC close green?"
    );
    assert_eq!(terminal.wallet_order_symbol_label("", "@107"), "HYPE/USDC");
}

#[test]
fn wallet_order_symbol_label_keeps_perp_and_hip3_keys_unchanged() {
    let terminal = TradingTerminal::boot().0;

    assert_eq!(terminal.wallet_order_symbol_label("", "BTC"), "BTC");
    assert_eq!(
        terminal.wallet_order_symbol_label("flex", "GOLD"),
        "flex:GOLD"
    );
    assert_eq!(
        terminal.wallet_order_symbol_label("flex", "flex:GOLD"),
        "flex:GOLD"
    );
}
