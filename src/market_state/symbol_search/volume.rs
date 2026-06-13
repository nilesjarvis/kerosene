use crate::api::{ExchangeSymbol, WatchlistContext};
use crate::helpers::finite_value;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Volume
// ---------------------------------------------------------------------------

pub(super) fn symbol_search_volume(
    contexts: &HashMap<String, WatchlistContext>,
    symbol: &ExchangeSymbol,
) -> Option<f64> {
    contexts
        .get(&symbol.key)
        .or_else(|| {
            (symbol.key == symbol.ticker)
                .then(|| contexts.get(&symbol.ticker))
                .flatten()
        })
        .and_then(|ctx| ctx.day_vlm)
        .and_then(finite_value)
}

pub(super) fn format_symbol_search_volume(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("${:.1}K", value / 1_000.0)
    } else {
        format!("${value:.0}")
    }
}
