use crate::api::{ExchangeSymbol, MarketType, spot_symbol_for_indexed_key};

#[cfg(test)]
mod tests;

pub(super) fn resolve_exchange_symbol<'a>(
    symbols: &'a [ExchangeSymbol],
    key_or_ticker: &str,
) -> Option<&'a ExchangeSymbol> {
    symbols
        .iter()
        .find(|symbol| symbol.key == key_or_ticker)
        // Persisted charts and watchlists may still carry the legacy
        // "@{index}" key for spot pairs the API names directly (PURR/USDC);
        // resolving the alias re-keys them to the canonical coin on load.
        .or_else(|| spot_symbol_for_indexed_key(symbols, key_or_ticker))
        .or_else(|| {
            symbols.iter().find(|symbol| {
                symbol.ticker == key_or_ticker && symbol.market_type == MarketType::Perp
            })
        })
        .or_else(|| symbols.iter().find(|symbol| symbol.ticker == key_or_ticker))
}
