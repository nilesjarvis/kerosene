use super::markets::{symbol_search_exchange_rank, symbol_search_matches_market_filter};
use super::volume::symbol_search_volume;
use crate::api::{ExchangeSymbol, WatchlistContext};
use crate::market_state::{SymbolSearchMarketFilter, SymbolSearchSortMode};

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Result Cache
// ---------------------------------------------------------------------------

pub(super) struct SymbolSearchResultsInput<'a, F>
where
    F: Fn(&ExchangeSymbol) -> bool,
{
    pub(super) symbols: &'a [ExchangeSymbol],
    pub(super) query: &'a str,
    pub(super) sort_mode: SymbolSearchSortMode,
    pub(super) market_filter: SymbolSearchMarketFilter,
    pub(super) hip3_dex_filter: Option<&'a str>,
    pub(super) favourite_symbols: &'a [String],
    pub(super) contexts: &'a HashMap<String, WatchlistContext>,
    pub(super) is_muted: F,
}

pub(super) fn filtered_symbol_search_indices<F>(
    input: SymbolSearchResultsInput<'_, F>,
) -> (Vec<usize>, usize)
where
    F: Fn(&ExchangeSymbol) -> bool,
{
    let query = input.query.to_lowercase();
    let mut indices: Vec<usize> = input
        .symbols
        .iter()
        .enumerate()
        .filter(|(_, symbol)| {
            symbol_search_matches_market_filter(symbol, input.market_filter, input.hip3_dex_filter)
        })
        .filter(|(_, symbol)| !(input.is_muted)(symbol))
        .filter(|(_, symbol)| symbol_search_query_matches(symbol, &query))
        .map(|(index, _)| index)
        .collect();

    indices.sort_by(|a_index, b_index| {
        let a = &input.symbols[*a_index];
        let b = &input.symbols[*b_index];
        let a_fav = input.favourite_symbols.iter().position(|key| key == &a.key);
        let b_fav = input.favourite_symbols.iter().position(|key| key == &b.key);

        match (a_fav, b_fav) {
            (Some(ai), Some(bi)) => return ai.cmp(&bi),
            (Some(_), None) => return std::cmp::Ordering::Less,
            (None, Some(_)) => return std::cmp::Ordering::Greater,
            (None, None) => {}
        }

        let fallback = || symbol_search_fallback_order(a, b, &query);
        match input.sort_mode {
            SymbolSearchSortMode::Relevance => fallback(),
            SymbolSearchSortMode::Alphabetical => {
                a.ticker.cmp(&b.ticker).then_with(|| a.key.cmp(&b.key))
            }
            SymbolSearchSortMode::Exchange => symbol_search_exchange_rank(a)
                .cmp(&symbol_search_exchange_rank(b))
                .then_with(fallback),
            SymbolSearchSortMode::Volume24h => {
                match (
                    symbol_search_volume(input.contexts, a),
                    symbol_search_volume(input.contexts, b),
                ) {
                    (Some(a_volume), Some(b_volume)) => b_volume
                        .partial_cmp(&a_volume)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(fallback),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => fallback(),
                }
            }
        }
    });

    let favourite_count = indices
        .iter()
        .filter(|index| {
            input
                .favourite_symbols
                .contains(&input.symbols[**index].key)
        })
        .count();
    (indices, favourite_count)
}

fn symbol_search_query_matches(sym: &ExchangeSymbol, query: &str) -> bool {
    query.is_empty()
        || sym.ticker.to_lowercase().contains(query)
        || sym.category.to_lowercase().contains(query)
        || sym
            .display_name
            .as_ref()
            .is_some_and(|dn| dn.to_lowercase().contains(query))
        || sym
            .keywords
            .iter()
            .any(|kw| kw.to_lowercase().contains(query))
        || sym.key.to_lowercase().contains(query)
}

fn symbol_search_fallback_order(
    a: &ExchangeSymbol,
    b: &ExchangeSymbol,
    query: &str,
) -> std::cmp::Ordering {
    let score_order = symbol_search_score(a, query).cmp(&symbol_search_score(b, query));
    if score_order != std::cmp::Ordering::Equal {
        score_order
    } else {
        a.ticker.cmp(&b.ticker).then_with(|| a.key.cmp(&b.key))
    }
}

fn symbol_search_score(sym: &ExchangeSymbol, query: &str) -> u8 {
    if query.is_empty() {
        return 0;
    }

    let ticker = sym.ticker.to_lowercase();
    let display_name = sym.display_name.as_deref().unwrap_or("").to_lowercase();
    let key = sym.key.to_lowercase();

    if ticker == query || display_name == query || key == query {
        0
    } else if ticker.starts_with(query) || display_name.starts_with(query) || key.starts_with(query)
    {
        1
    } else {
        2
    }
}
