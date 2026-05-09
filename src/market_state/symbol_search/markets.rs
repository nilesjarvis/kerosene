use crate::api::{ExchangeSymbol, MarketType};
use crate::market_state::SymbolSearchMarketFilter;

use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Markets
// ---------------------------------------------------------------------------

pub(super) fn symbol_search_hip3_dexes(symbols: &[ExchangeSymbol]) -> Vec<String> {
    let mut dexes = BTreeSet::new();
    for symbol in symbols {
        if symbol.market_type == MarketType::Perp
            && let Some((dex, _)) = symbol.key.split_once(':')
        {
            dexes.insert(dex.to_string());
        }
    }
    dexes.into_iter().collect()
}

pub(super) fn symbol_search_matches_market_filter(
    symbol: &ExchangeSymbol,
    filter: SymbolSearchMarketFilter,
    hip3_dex_filter: Option<&str>,
) -> bool {
    match filter {
        SymbolSearchMarketFilter::All => true,
        SymbolSearchMarketFilter::NativePerps => {
            symbol.market_type == MarketType::Perp && !symbol.key.contains(':')
        }
        SymbolSearchMarketFilter::Spot => symbol.market_type == MarketType::Spot,
        SymbolSearchMarketFilter::Hip3 => {
            if symbol.market_type != MarketType::Perp {
                return false;
            }
            let Some((dex, _)) = symbol.key.split_once(':') else {
                return false;
            };
            hip3_dex_filter.is_none_or(|selected| selected == dex)
        }
        SymbolSearchMarketFilter::Outcomes => symbol.market_type == MarketType::Outcome,
    }
}

pub(super) fn symbol_search_exchange_label(symbol: &ExchangeSymbol) -> String {
    match symbol.market_type {
        MarketType::Perp => {
            if let Some((dex, _)) = symbol.key.split_once(':') {
                format!("HIP-3: {dex}")
            } else {
                "Native Perps".to_string()
            }
        }
        MarketType::Spot => "Spot".to_string(),
        MarketType::Outcome => "Outcomes".to_string(),
    }
}

pub(super) fn symbol_search_exchange_rank(symbol: &ExchangeSymbol) -> (u8, String) {
    match symbol.market_type {
        MarketType::Perp => {
            if let Some((dex, _)) = symbol.key.split_once(':') {
                (2, dex.to_string())
            } else {
                (0, String::new())
            }
        }
        MarketType::Spot => (1, String::new()),
        MarketType::Outcome => (3, String::new()),
    }
}
