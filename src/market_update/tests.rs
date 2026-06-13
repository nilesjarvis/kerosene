use super::*;
use crate::market_state::OrderBookDisplayMode;

#[test]
fn order_book_market_dispatch_includes_order_book_controls() {
    let source_context = crate::read_data_provider::MarketDataSourceContext {
        provider: crate::config::ReadDataProvider::Hyperliquid,
        read_data_provider_generation: 0,
        hydromancer_key_generation: None,
    };

    assert!(is_order_book_market_message(
        &Message::SetOrderBookDisplayMode(7, OrderBookDisplayMode::DomLadder)
    ));
    assert!(is_order_book_market_message(
        &Message::ToggleOrderBookCenterOnMid(7)
    ));
    assert!(is_order_book_market_message(
        &Message::ToggleOrderBookReverseSide(7)
    ));
    assert!(is_order_book_market_message(
        &Message::OrderBookWsBookLagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: (Some(5), None),
            source_context,
            skipped: 9,
        }
    ));
    assert!(is_order_book_market_message(
        &Message::OrderBookWsAssetCtxLagged {
            id: 7,
            coin: "BTC".to_string(),
            source_context,
            skipped: 9,
        }
    ));
}

#[test]
fn positioning_market_dispatch_includes_asset_context_lag() {
    assert!(is_positioning_info_market_message(
        &Message::PositioningInfoWsAssetCtxLagged(
            "BTC".to_string(),
            crate::read_data_provider::MarketDataSourceContext {
                provider: crate::config::ReadDataProvider::Hyperliquid,
                read_data_provider_generation: 0,
                hydromancer_key_generation: None,
            },
            9,
        )
    ));
}

#[test]
fn live_watchlist_market_dispatch_includes_settings_toggle() {
    assert!(is_live_watchlist_market_message(
        &Message::ToggleLiveWatchlistSettings(7)
    ));
}
