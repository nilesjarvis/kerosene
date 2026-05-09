use crate::api::ExchangeSymbol;
use std::cmp::Ordering;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Chart Editor Search
// ---------------------------------------------------------------------------

pub(super) fn chart_editor_symbol_matches(symbol: &ExchangeSymbol, query: &str) -> bool {
    query.is_empty()
        || symbol.ticker.to_lowercase().contains(query)
        || symbol.category.to_lowercase().contains(query)
        || symbol
            .display_name
            .as_ref()
            .is_some_and(|display_name| display_name.to_lowercase().contains(query))
        || symbol
            .keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(query))
        || symbol.key.to_lowercase().contains(query)
}

pub(super) fn chart_editor_symbol_score(symbol: &ExchangeSymbol, query: &str) -> u8 {
    if query.is_empty() {
        return 0;
    }

    let ticker = symbol.ticker.to_lowercase();
    let display_name = symbol.display_name.as_deref().unwrap_or("").to_lowercase();
    let key = symbol.key.to_lowercase();

    if ticker == query || display_name == query || key == query {
        0
    } else if ticker.starts_with(query) || display_name.starts_with(query) || key.starts_with(query)
    {
        1
    } else {
        2
    }
}

pub(super) fn compare_chart_editor_symbols(
    a: &ExchangeSymbol,
    b: &ExchangeSymbol,
    query: &str,
    favourite_symbols: &[String],
) -> Ordering {
    if !query.is_empty() {
        let score_order =
            chart_editor_symbol_score(a, query).cmp(&chart_editor_symbol_score(b, query));
        if score_order != Ordering::Equal {
            return score_order;
        }
    }

    let a_fav = favourite_symbols.iter().position(|key| key == &a.key);
    let b_fav = favourite_symbols.iter().position(|key| key == &b.key);
    match (a_fav, b_fav) {
        (Some(ai), Some(bi)) => ai.cmp(&bi),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.ticker.cmp(&b.ticker).then_with(|| a.key.cmp(&b.key)),
    }
}
