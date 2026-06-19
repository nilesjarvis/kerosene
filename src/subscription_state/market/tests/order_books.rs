use super::*;

#[test]
fn outcome_order_books_subscribe_to_l2_without_asset_ctx() {
    assert_eq!(
        order_book_market_streams_for_symbol("#650", false, true),
        OrderBookMarketStreams {
            l2_book: true,
            asset_ctx: false,
        }
    );
}

#[test]
fn non_outcome_order_books_subscribe_to_l2_and_asset_ctx() {
    assert_eq!(
        order_book_market_streams_for_symbol("BTC", false, false),
        OrderBookMarketStreams {
            l2_book: true,
            asset_ctx: true,
        }
    );
}

#[test]
fn hidden_or_empty_order_books_do_not_subscribe() {
    let disabled = OrderBookMarketStreams {
        l2_book: false,
        asset_ctx: false,
    };

    assert_eq!(
        order_book_market_streams_for_symbol("", false, false),
        disabled
    );
    assert_eq!(
        order_book_market_streams_for_symbol("BTC", true, false),
        disabled
    );
}

#[test]
fn order_book_lagged_stream_event_maps_to_market_message() {
    let terminal = TradingTerminal::boot().0;
    let source_context = terminal.market_data_source_context();

    let message = order_book_stream_event_message((
        source_context,
        crate::ws::KeyedBookStreamEvent::Lagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: (Some(5), None),
            hydromancer_key_generation: source_context.hydromancer_key_generation,
            skipped: 9,
        },
    ));

    match message {
        Message::OrderBookWsBookLagged {
            id,
            coin,
            sigfigs,
            source_context: mapped_context,
            skipped,
        } => {
            assert_eq!(id, 7);
            assert_eq!(coin, "BTC");
            assert_eq!(sigfigs, (Some(5), None));
            assert_eq!(mapped_context, source_context);
            assert_eq!(skipped, 9);
        }
        other => panic!("expected order-book lagged message, got {other:?}"),
    }
}

#[test]
fn order_book_lagged_stream_event_preserves_fallback_generation_scope() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.read_data_provider = ReadDataProvider::Hydromancer;
    terminal.hydromancer_api_key = "hydro-secret".to_string().into();
    terminal.hydromancer_key_generation = 2;
    let source_context = terminal.market_data_source_context();
    assert_eq!(source_context.hydromancer_key_generation, Some(2));

    let message = order_book_stream_event_message((
        source_context,
        crate::ws::KeyedBookStreamEvent::Lagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: (Some(5), None),
            hydromancer_key_generation: None,
            skipped: 9,
        },
    ));

    match message {
        Message::OrderBookWsBookLagged {
            source_context: mapped_context,
            ..
        } => {
            assert_eq!(
                mapped_context,
                crate::read_data_provider::MarketDataSourceContext {
                    hydromancer_key_generation: None,
                    ..source_context
                }
            );
        }
        other => panic!("expected order-book lagged message, got {other:?}"),
    }
}

#[test]
fn order_book_asset_context_lagged_event_maps_to_market_message() {
    let terminal = TradingTerminal::boot().0;
    let source_context = terminal.market_data_source_context();

    let message = order_book_asset_ctx_stream_event_message((
        source_context,
        crate::ws::KeyedAssetContextStreamEvent::Lagged {
            id: 7,
            symbol: "BTC".to_string(),
            hydromancer_key_generation: source_context.hydromancer_key_generation,
            skipped: 9,
        },
    ));

    match message {
        Message::OrderBookWsAssetCtxLagged {
            id,
            coin,
            source_context: mapped_context,
            skipped,
        } => {
            assert_eq!(id, 7);
            assert_eq!(coin, "BTC");
            assert_eq!(mapped_context, source_context);
            assert_eq!(skipped, 9);
        }
        other => panic!("expected order-book asset-context lagged message, got {other:?}"),
    }
}
