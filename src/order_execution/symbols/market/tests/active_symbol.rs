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
fn switch_to_non_tradable_outcome_reports_display_label_not_raw_key() {
    let mut terminal = TradingTerminal::boot().0;
    let fallback = outcome_symbol("#660", true);
    let label = TradingTerminal::exchange_symbol_display_name(&fallback);
    terminal.exchange_symbols = vec![fallback, symbol("HYPE", MarketType::Perp)];

    let _task = terminal.switch_active_symbol_internal("#660".to_string());

    let (message, is_error) = match &terminal.order_status {
        Some((message, is_error)) => (message.clone(), *is_error),
        None => panic!("status should be set"),
    };
    assert!(is_error);
    assert_eq!(message, format!("{label} is not a tradable market"));
    assert!(!message.contains("#660"));
    match &terminal.symbol_search_status {
        Some((search_message, _)) => assert_eq!(search_message, &message),
        None => panic!("search status should be set"),
    }
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
