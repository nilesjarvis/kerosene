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
