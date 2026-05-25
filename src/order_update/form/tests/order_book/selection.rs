use super::*;
use crate::api::MarketType;
use crate::signing::OrderKind;

#[test]
fn order_book_price_selected_sets_limit_price_for_active_symbol_book() {
    let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);

    let _task = terminal.handle_order_book_price_selected(7, "100".to_string());

    assert_eq!(terminal.active_symbol, "BTC");
    assert_eq!(terminal.order_kind, OrderKind::Limit);
    assert_eq!(terminal.order_price, "100");
}

#[test]
fn order_book_price_selected_switches_to_fixed_symbol_before_setting_price() {
    let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Fixed("ETH".to_string()));
    terminal.exchange_symbols = vec![
        symbol("BTC", MarketType::Perp),
        symbol("ETH", MarketType::Perp),
    ];
    terminal.order_quantity = "10".to_string();
    terminal.order_percentage = 50.0;

    let _task = terminal.handle_order_book_price_selected(7, "2500.5".to_string());

    assert_eq!(terminal.active_symbol, "ETH");
    assert_eq!(terminal.active_symbol_display, "ETH");
    assert_eq!(terminal.order_kind, OrderKind::Limit);
    assert_eq!(terminal.order_price, "2500.5");
    assert!(terminal.order_quantity.is_empty());
    assert_eq!(terminal.order_percentage, 0.0);
}

#[test]
fn order_book_price_selected_rejects_missing_book_without_mutating_form() {
    let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);

    let _task = terminal.handle_order_book_price_selected(99, "101.25".to_string());

    assert_eq!(terminal.active_symbol, "BTC");
    assert_eq!(terminal.order_kind, OrderKind::Market);
    assert!(terminal.order_price.is_empty());
    assert_eq!(
        terminal.order_status,
        Some(("Order book unavailable".to_string(), true))
    );
}

#[test]
fn order_book_price_selected_rejects_invalid_price_without_mutating_form() {
    let mut terminal = terminal_with_order_book(OrderBookSymbolMode::Active);

    let _task = terminal.handle_order_book_price_selected(7, "0".to_string());

    assert_eq!(terminal.active_symbol, "BTC");
    assert_eq!(terminal.order_kind, OrderKind::Market);
    assert!(terminal.order_price.is_empty());
    assert_eq!(
        terminal.order_status,
        Some(("Invalid order-book price".to_string(), true))
    );
}
