use super::*;

#[test]
fn order_book_track_check_matches_active_or_fixed_symbol() {
    assert!(order_book_tracks_coin(
        &OrderBookSymbolMode::Active,
        "BTC",
        "BTC"
    ));
    assert!(!order_book_tracks_coin(
        &OrderBookSymbolMode::Active,
        "ETH",
        "BTC"
    ));
    assert!(order_book_tracks_coin(
        &OrderBookSymbolMode::Fixed("BTC".to_string()),
        "ETH",
        "BTC"
    ));
    assert!(!order_book_tracks_coin(
        &OrderBookSymbolMode::Fixed("ETH".to_string()),
        "BTC",
        "BTC"
    ));
}
