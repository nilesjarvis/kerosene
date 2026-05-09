use crate::api::{ExchangeSymbol, MarketType};

#[cfg(test)]
mod tests;

pub(super) fn resolve_exchange_symbol<'a>(
    symbols: &'a [ExchangeSymbol],
    key_or_ticker: &str,
) -> Option<&'a ExchangeSymbol> {
    symbols
        .iter()
        .find(|symbol| symbol.key == key_or_ticker)
        .or_else(|| {
            symbols.iter().find(|symbol| {
                symbol.ticker == key_or_ticker && symbol.market_type == MarketType::Perp
            })
        })
        .or_else(|| symbols.iter().find(|symbol| symbol.ticker == key_or_ticker))
}
