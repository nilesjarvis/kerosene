// ---------------------------------------------------------------------------
// Order Book Market Streams
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrderBookMarketStreams {
    pub(super) l2_book: bool,
    pub(super) asset_ctx: bool,
}

pub(super) fn order_book_market_streams_for_symbol(
    symbol: &str,
    hidden: bool,
    outcome: bool,
) -> OrderBookMarketStreams {
    let market_data_enabled = !symbol.is_empty() && !hidden;
    OrderBookMarketStreams {
        l2_book: market_data_enabled,
        asset_ctx: market_data_enabled && !outcome,
    }
}
