use super::*;
use crate::market_state::OrderBookDisplayMode;

#[test]
fn order_book_market_dispatch_includes_order_book_controls() {
    assert!(is_order_book_market_message(
        &Message::SetOrderBookDisplayMode(7, OrderBookDisplayMode::DomLadder)
    ));
    assert!(is_order_book_market_message(
        &Message::ToggleOrderBookCenterOnMid(7)
    ));
    assert!(is_order_book_market_message(
        &Message::ToggleOrderBookReverseSide(7)
    ));
}

#[test]
fn live_watchlist_market_dispatch_includes_settings_toggle() {
    assert!(is_live_watchlist_market_message(
        &Message::ToggleLiveWatchlistSettings(7)
    ));
}
