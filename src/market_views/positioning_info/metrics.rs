use crate::api::ExchangeSymbol;

mod change;
mod formatting;
mod live;
pub(super) use change::*;
pub(super) use formatting::*;
pub(super) use live::*;

pub(super) fn positioning_symbol_matches(symbol: &ExchangeSymbol, query: &str) -> bool {
    symbol.key.to_lowercase().contains(query)
        || symbol.ticker.to_lowercase().contains(query)
        || symbol
            .display_name
            .as_deref()
            .is_some_and(|display| display.to_lowercase().contains(query))
        || symbol
            .keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(query))
}
