use crate::api::{ExchangeSymbol, WatchlistContext};

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
        .or_else(|| contexts.get(&symbol.ticker))
        .and_then(|ctx| ctx.day_vlm)
        .filter(|value| value.is_finite())
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
