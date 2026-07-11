use super::*;
use crate::api::{BookLevel, OrderBook};

fn instance(mode: OrderBookSymbolMode) -> OrderBookInstance {
    let mut inst = OrderBookInstance::new(7, mode, 1.0);
    inst.set_book(OrderBook {
        bids: vec![BookLevel { px: 99.0, sz: 1.0 }],
        asks: vec![BookLevel { px: 101.0, sz: 1.0 }],
    });
    inst
}

#[test]
fn selection_uses_active_or_fixed_symbol_and_trims_price() {
    let active = instance(OrderBookSymbolMode::Active);
    let fixed = instance(OrderBookSymbolMode::Fixed(" ETH ".to_string()));

    assert_eq!(
        order_book_price_selection(Some(&active), "BTC", " 100.5 "),
        Ok(OrderBookPriceSelection {
            selected_price: "100.5".to_string(),
            target_symbol: "BTC".to_string(),
        })
    );
    assert_eq!(
        order_book_price_selection(Some(&fixed), "BTC", "2500"),
        Ok(OrderBookPriceSelection {
            selected_price: "2500".to_string(),
            target_symbol: "ETH".to_string(),
        })
    );
}

#[test]
fn selection_rejects_invalid_price_before_book_availability() {
    assert_eq!(
        order_book_price_selection(None, "BTC", "0"),
        Err(OrderBookPriceSelectionError::InvalidPrice)
    );
}

#[test]
fn selection_rejects_missing_or_empty_fixed_books() {
    let empty_fixed = instance(OrderBookSymbolMode::Fixed(" ".to_string()));

    assert_eq!(
        order_book_price_selection(None, "BTC", "100"),
        Err(OrderBookPriceSelectionError::Unavailable)
    );
    assert_eq!(
        order_book_price_selection(Some(&empty_fixed), "BTC", "100"),
        Err(OrderBookPriceSelectionError::Unavailable)
    );
}

#[test]
fn selection_debug_redacts_price_and_symbol_without_changing_them() {
    let selection = OrderBookPriceSelection {
        selected_price: "98765.4321".to_string(),
        target_symbol: "private-book-symbol-sentinel".to_string(),
    };

    let rendered = format!("{selection:?}");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains("98765.4321"), "{rendered}");
    assert!(
        !rendered.contains("private-book-symbol-sentinel"),
        "{rendered}"
    );
    assert_eq!(selection.selected_price, "98765.4321");
    assert_eq!(selection.target_symbol, "private-book-symbol-sentinel");
}
