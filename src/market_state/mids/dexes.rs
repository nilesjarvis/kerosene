use crate::api::{ExchangeSymbol, MarketType};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// All-Mids Dex Discovery
// ---------------------------------------------------------------------------

pub(super) fn known_mids_dexes(
    symbols: &[ExchangeSymbol],
    known_hip3_dexes: &[&str],
) -> Vec<String> {
    let mut dexes = vec![String::new()];
    for dex in known_hip3_dexes {
        dexes.push((*dex).to_string());
    }
    for symbol in symbols {
        if symbol.market_type == MarketType::Perp
            && let Some((dex, _)) = symbol.key.split_once(':')
        {
            dexes.push(dex.to_string());
        }
    }
    dexes.sort();
    dexes.dedup();
    if let Some(main_idx) = dexes.iter().position(|dex| dex.is_empty()) {
        dexes.swap(0, main_idx);
    }
    dexes
}
