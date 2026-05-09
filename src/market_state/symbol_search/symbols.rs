use crate::api::ExchangeSymbol;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Context Symbols
// ---------------------------------------------------------------------------

pub(super) fn context_symbol_keys<'a, I, MatchesMarket, IsMuted>(
    symbols: I,
    mut matches_market: MatchesMarket,
    mut is_muted: IsMuted,
) -> Vec<String>
where
    I: IntoIterator<Item = &'a ExchangeSymbol>,
    MatchesMarket: FnMut(&ExchangeSymbol) -> bool,
    IsMuted: FnMut(&ExchangeSymbol) -> bool,
{
    let mut keys: Vec<String> = symbols
        .into_iter()
        .filter(|symbol| matches_market(symbol))
        .filter(|symbol| !is_muted(symbol))
        .map(|symbol| symbol.key.clone())
        .collect();

    keys.sort();
    keys.dedup();
    keys
}
