use super::*;

#[test]
fn outcome_symbols_are_available_for_order_book_fetches() {
    let terminal = TradingTerminal::boot().0;

    assert_eq!(terminal.order_book_unavailable_reason("#650"), None);
}

#[test]
fn mode_resolution_preserves_literal_symbols_and_distinct_empty_hidden_states() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = " BTC ".to_string();
    assert_eq!(
        terminal.order_book_symbol_for_mode(&OrderBookSymbolMode::Active),
        " BTC "
    );
    assert_eq!(
        terminal.order_book_symbol_for_mode(&OrderBookSymbolMode::Fixed("#650".to_string())),
        "#650"
    );
    assert_eq!(
        terminal.order_book_symbol_for_mode(&OrderBookSymbolMode::Fixed(String::new())),
        ""
    );
    terminal.order_books.clear();
    terminal.order_books.insert(
        7,
        OrderBookInstance::new(7, OrderBookSymbolMode::Active, 1.0),
    );
    terminal.order_books.insert(
        8,
        OrderBookInstance::new(8, OrderBookSymbolMode::Fixed("xyz:ETH".to_string()), 1.0),
    );
    terminal.active_symbol.clear();
    assert_eq!(
        terminal.order_book_unavailable_reason("").as_deref(),
        Some("No order-book symbol selected")
    );
    assert!(!terminal.order_book_instance_is_muted(7));
    assert!(!terminal.order_book_instance_is_muted(99));
    terminal.active_symbol = "BTC".to_string();
    terminal.muted_tickers.insert("BTC".to_string());
    assert!(terminal.order_book_instance_is_muted(7));
    assert!(!terminal.order_book_instance_is_muted(8));
    assert_eq!(
        terminal.order_book_unavailable_reason("BTC").as_deref(),
        Some("Order book ticker is hidden in Settings > Risk")
    );
    terminal.muted_tickers.clear();
    terminal.market_universe = crate::config::MarketUniverseConfig::Hip3Dex {
        dex: "xyz".to_string(),
    };
    assert!(terminal.order_book_instance_is_muted(7));
    assert!(!terminal.order_book_instance_is_muted(8));
}
