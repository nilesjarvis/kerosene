use crate::api::{ExchangeSymbol, MarketType, spot_symbol_for_indexed_key};
use crate::app_state::TradingTerminal;

// ---------------------------------------------------------------------------
// Market Symbol Lookup
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// Exact key match plus the legacy "@{index}" alias for spot pairs the
    /// API names directly (PURR/USDC), so persisted keys saved before the
    /// pair was re-keyed keep resolving.
    pub(crate) fn exchange_symbol_for_key(&self, key: &str) -> Option<&ExchangeSymbol> {
        self.exchange_symbols
            .iter()
            .find(|s| s.key == key)
            .or_else(|| spot_symbol_for_indexed_key(&self.exchange_symbols, key))
    }

    pub(crate) fn resolve_exchange_symbol_by_key_or_ticker(
        &self,
        key_or_ticker: &str,
    ) -> Option<&ExchangeSymbol> {
        self.exchange_symbol_for_key(key_or_ticker)
            .or_else(|| {
                self.exchange_symbols
                    .iter()
                    .find(|s| s.ticker == key_or_ticker && s.market_type == MarketType::Perp)
            })
            .or_else(|| {
                self.exchange_symbols
                    .iter()
                    .find(|s| s.ticker == key_or_ticker)
            })
    }

    pub(crate) fn exchange_symbol_is_orderable(&self, symbol: &ExchangeSymbol) -> bool {
        symbol.is_user_selectable_market()
            && !self.exchange_symbol_is_hidden(symbol)
            && (symbol.market_type != MarketType::Outcome || symbol.outcome.is_some())
    }

    pub(crate) fn validate_exchange_symbol_orderable(
        &self,
        symbol: &ExchangeSymbol,
        hidden_context: &str,
    ) -> Result<(), String> {
        if symbol.market_type == MarketType::Spot && self.spot_metadata_degraded {
            return Err(format!(
                "{} spot metadata is temporarily unverified; spot trading is disabled until it refreshes",
                Self::exchange_symbol_display_name(symbol)
            ));
        }
        if symbol.market_type == MarketType::Outcome && symbol.outcome.is_none() {
            return Err(format!(
                "{} outcome metadata is incomplete",
                Self::exchange_symbol_display_name(symbol)
            ));
        }
        if !symbol.is_user_selectable_market() {
            return Err(format!(
                "{} is not a tradable market",
                Self::exchange_symbol_display_name(symbol)
            ));
        }
        if self.exchange_symbol_is_hidden(symbol) {
            return Err(format!(
                "{hidden_context} ticker is hidden in Settings > Risk"
            ));
        }
        Ok(())
    }

    pub(crate) fn restored_active_symbol_key(&self, requested: &str) -> Option<String> {
        let requested = match requested.trim() {
            "" => "HYPE",
            requested => requested,
        };

        if self.exchange_symbols.is_empty() {
            return (!self.symbol_key_is_hidden(requested))
                .then(|| requested.to_string())
                .or_else(|| self.fallback_unmuted_symbol_key());
        }

        self.resolve_exchange_symbol_by_key_or_ticker(requested)
            .filter(|symbol| self.exchange_symbol_is_orderable(symbol))
            .map(|symbol| symbol.key.clone())
            .or_else(|| self.fallback_unmuted_symbol_key())
    }

    pub(crate) fn market_type_is_spot_like(market_type: MarketType) -> bool {
        matches!(market_type, MarketType::Spot | MarketType::Outcome)
    }

    pub(crate) fn market_type_for_symbol(&self, coin: &str) -> Option<MarketType> {
        let _theme = self.theme();
        self.exchange_symbols
            .iter()
            .find(|s| s.key == coin)
            .map(|s| s.market_type)
    }

    /// Check whether a given coin key is a spot market.
    pub(crate) fn is_spot_coin(&self, coin: &str) -> bool {
        self.market_type_for_symbol(coin) == Some(MarketType::Spot)
    }

    pub(crate) fn is_outcome_coin(&self, coin: &str) -> bool {
        self.market_type_for_symbol(coin) == Some(MarketType::Outcome) || coin.starts_with('#')
    }

    pub(crate) fn is_perp_coin(&self, coin: &str) -> bool {
        self.market_type_for_symbol(coin) == Some(MarketType::Perp)
            || (!coin.starts_with('@')
                && !coin.starts_with('#')
                // API-named spot pairs carry a "{base}/{quote}" slash.
                && !coin.contains('/')
                && self.market_type_for_symbol(coin).is_none())
    }
}
