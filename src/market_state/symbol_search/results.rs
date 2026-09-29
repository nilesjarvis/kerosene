use super::markets::{symbol_search_exchange_rank, symbol_search_matches_market_filter};
use super::volume::symbol_search_volume;
use crate::api::{ExchangeSymbol, WatchlistContext};
use crate::helpers::compare_symbol_keys_for_same_ticker;
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

struct RankedSymbol {
    index: usize,
    favourite_rank: Option<usize>,
    relevance: u8,
    volume: Option<f64>,
}

pub(super) fn filtered_symbol_search_indices<F>(
    input: SymbolSearchResultsInput<'_, F>,
) -> (Vec<usize>, usize)
where
    F: Fn(&ExchangeSymbol) -> bool,
{
    let query = input.query.to_lowercase();
    let mut favourite_ranks = HashMap::new();
    for (rank, key) in input.favourite_symbols.iter().enumerate() {
        favourite_ranks.entry(key.as_str()).or_insert(rank);
    }
    let mut ranked: Vec<RankedSymbol> = input
        .symbols
        .iter()
        .enumerate()
        .filter(|(_, symbol)| {
            symbol_search_matches_market_filter(symbol, input.market_filter, input.hip3_dex_filter)
        })
        .filter(|(_, symbol)| !(input.is_muted)(symbol))
        .filter(|(_, symbol)| symbol_search_query_matches(symbol, &query))
        .map(|(index, symbol)| {
            let favourite_rank = favourite_ranks.get(symbol.key.as_str()).copied();
            // Favourites use only their saved order. Other rows compute each
            // ranking value once, and only when the selected sort needs it.
            let relevance = if favourite_rank.is_none()
                && input.sort_mode != SymbolSearchSortMode::Alphabetical
            {
                symbol_search_score(symbol, &query)
            } else {
                0
            };
            let volume =
                if favourite_rank.is_none() && input.sort_mode == SymbolSearchSortMode::Volume24h {
                    symbol_search_volume(input.contexts, symbol)
                } else {
                    None
                };
            RankedSymbol {
                index,
                favourite_rank,
                relevance,
                volume,
            }
        })
        .collect();

    ranked.sort_by(|a, b| {
        match (a.favourite_rank, b.favourite_rank) {
            (Some(ai), Some(bi)) => return ai.cmp(&bi),
            (Some(_), None) => return std::cmp::Ordering::Less,
            (None, Some(_)) => return std::cmp::Ordering::Greater,
            (None, None) => {}
        }

        let a_symbol = &input.symbols[a.index];
        let b_symbol = &input.symbols[b.index];
        let alphabetical = || {
            a_symbol
                .ticker
                .cmp(&b_symbol.ticker)
                .then_with(|| compare_symbol_keys_for_same_ticker(&a_symbol.key, &b_symbol.key))
        };
        let fallback = || a.relevance.cmp(&b.relevance).then_with(alphabetical);
        match input.sort_mode {
            SymbolSearchSortMode::Relevance => fallback(),
            SymbolSearchSortMode::Alphabetical => alphabetical(),
            SymbolSearchSortMode::Exchange => symbol_search_exchange_rank(a_symbol)
                .cmp(&symbol_search_exchange_rank(b_symbol))
                .then_with(fallback),
            SymbolSearchSortMode::Volume24h => match (a.volume, b.volume) {
                (Some(a_volume), Some(b_volume)) => b_volume
                    .partial_cmp(&a_volume)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(fallback),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => fallback(),
            },
        }
    });

    let favourite_count = ranked
        .iter()
        .take_while(|symbol| symbol.favourite_rank.is_some())
        .count();
    let indices = ranked.into_iter().map(|symbol| symbol.index).collect();
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
