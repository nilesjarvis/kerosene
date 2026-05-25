use crate::api::MarketType;
use crate::app_state::TradingTerminal;

// ---------------------------------------------------------------------------
// HyperDash Symbol Mapping
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// Resolve a chart symbol key to the coin symbol expected by HyperDash.
    /// Returns None for symbols HyperDash does not support (spot/outcome markets).
    pub(crate) fn hyperdash_coin_for_symbol(&self, symbol_key: &str) -> Option<String> {
        let _theme = self.theme();
        if let Some(sym) = self.exchange_symbols.iter().find(|s| s.key == symbol_key) {
            return match sym.market_type {
                MarketType::Perp => Some(sym.key.clone()),
                MarketType::Spot | MarketType::Outcome => None,
            };
        }

        if symbol_key.starts_with('@') || symbol_key.starts_with('#') {
            None
        } else {
            Some(symbol_key.to_string())
        }
    }
}
