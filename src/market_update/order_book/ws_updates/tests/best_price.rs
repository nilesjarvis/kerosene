use super::*;

#[test]
fn best_chase_price_uses_bid_for_buy_and_ask_for_sell() {
    let book = book();

    assert_eq!(best_chase_price(&book, true), Some(99.0));
    assert_eq!(best_chase_price(&book, false), Some(101.0));
    assert_eq!(best_chase_price(&OrderBook::empty(), true), None);
    assert_eq!(best_chase_price(&OrderBook::empty(), false), None);
}

#[test]
fn best_chase_price_rejects_invalid_book_levels() {
    let invalid_book = OrderBook {
        bids: vec![BookLevel {
            px: f64::INFINITY,
            sz: 1.0,
        }],
        asks: vec![BookLevel { px: 0.0, sz: 1.0 }],
    };

    assert_eq!(best_chase_price(&invalid_book, true), None);
    assert_eq!(best_chase_price(&invalid_book, false), None);
}
