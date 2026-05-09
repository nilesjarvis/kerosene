use crate::market_state::{SYMBOL_SEARCH_ALL_HIP3_DEXES, SymbolSearchMarketFilter};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Controls
// ---------------------------------------------------------------------------

pub(super) fn toggle_favourite_symbol(favourites: &mut Vec<String>, key: String) {
    if let Some(pos) = favourites.iter().position(|candidate| candidate == &key) {
        favourites.remove(pos);
    } else {
        favourites.push(key);
    }
}

pub(super) fn apply_market_filter(
    current_filter: &mut SymbolSearchMarketFilter,
    hip3_dex_filter: &mut Option<String>,
    filter: SymbolSearchMarketFilter,
) {
    *current_filter = filter;
    if filter != SymbolSearchMarketFilter::Hip3 {
        *hip3_dex_filter = None;
    }
}

pub(super) fn apply_hip3_dex_filter(hip3_dex_filter: &mut Option<String>, dex: String) {
    *hip3_dex_filter = (dex != SYMBOL_SEARCH_ALL_HIP3_DEXES).then_some(dex);
}
