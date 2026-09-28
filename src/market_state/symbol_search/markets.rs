use crate::api::{ExchangeSymbol, MarketType};
use crate::market_state::SymbolSearchMarketFilter;

use std::borrow::Cow;
use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Markets
// ---------------------------------------------------------------------------

pub(super) fn symbol_search_hip3_dexes(symbols: &[ExchangeSymbol]) -> Vec<&str> {
    let mut dexes = BTreeSet::new();
    for symbol in symbols {
        if symbol.market_type == MarketType::Perp
            && let Some((dex, _)) = symbol.key.split_once(':')
        {
            dexes.insert(dex);
        }
    }
    dexes.into_iter().collect()
}

pub(super) fn symbol_search_matches_market_filter(
    symbol: &ExchangeSymbol,
    filter: SymbolSearchMarketFilter,
    hip3_dex_filter: Option<&str>,
) -> bool {
    if !symbol.is_user_selectable_market() {
        return false;
    }

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

pub(super) fn symbol_search_exchange_label(symbol: &ExchangeSymbol) -> Cow<'static, str> {
    match symbol.market_type {
        MarketType::Perp => {
            if let Some((dex, _)) = symbol.key.split_once(':') {
                format!("HIP-3: {dex}").into()
            } else {
                "Native Perps".into()
            }
        }
        MarketType::Spot => "Spot".into(),
        MarketType::Outcome => "Outcomes".into(),
    }
}

pub(super) fn symbol_search_exchange_rank(symbol: &ExchangeSymbol) -> (u8, &str) {
    match symbol.market_type {
        MarketType::Perp => {
            if let Some((dex, _)) = symbol.key.split_once(':') {
                (2, dex)
            } else {
                (0, "")
            }
        }
        MarketType::Spot => (1, ""),
        MarketType::Outcome => (3, ""),
    }
}
