use super::*;

#[test]
fn switch_active_symbol_clears_order_sizing_inputs() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        symbol("BTC", MarketType::Perp),
        symbol("ETH", MarketType::Perp),
    ];
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.order_quantity = "1500".to_string();
    terminal.order_percentage = 75.0;
    terminal.order_quantity_is_usd = true;

    let _task = terminal.switch_active_symbol_internal("ETH".to_string());

    assert_eq!(terminal.active_symbol, "ETH");
    assert_eq!(terminal.active_symbol_display, "ETH");
    assert!(terminal.order_quantity.is_empty());
    assert_eq!(terminal.order_percentage, 0.0);
    assert!(terminal.order_quantity_is_usd);
}

#[test]
fn restored_active_symbol_key_replaces_non_tradable_fallback_outcome() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![
        outcome_symbol("#660", true),
        outcome_symbol("#670", false),
        symbol("HYPE", MarketType::Perp),
    ];

    assert_eq!(
        terminal.restored_active_symbol_key("#660"),
        Some("HYPE".to_string())
    );
}
