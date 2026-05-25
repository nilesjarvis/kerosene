use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::MarketUniverseConfig;

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Market Universe Matching
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn symbol_matches_market_universe(
        symbol: &ExchangeSymbol,
        universe: &MarketUniverseConfig,
    ) -> bool {
        match universe {
            MarketUniverseConfig::All => true,
            MarketUniverseConfig::Hip3Dex { dex } => {
                symbol.market_type == MarketType::Perp
                    && symbol
                        .key
                        .split_once(':')
                        .is_some_and(|(symbol_dex, _)| symbol_dex.eq_ignore_ascii_case(dex))
            }
        }
    }

    pub(crate) fn key_matches_market_universe(
        exchange_symbols: &[ExchangeSymbol],
        universe: &MarketUniverseConfig,
        key_or_coin: &str,
    ) -> bool {
        match universe {
            MarketUniverseConfig::All => true,
            MarketUniverseConfig::Hip3Dex { dex } => {
                let key_or_coin = key_or_coin.trim();
                if key_or_coin.is_empty() {
                    return false;
                }

                if let Some(symbol) = exchange_symbols.iter().find(|symbol| {
                    symbol.key.eq_ignore_ascii_case(key_or_coin)
                        || symbol.ticker.eq_ignore_ascii_case(key_or_coin)
                }) {
                    return Self::symbol_matches_market_universe(symbol, universe);
                }

                key_or_coin
                    .split_once(':')
                    .is_some_and(|(symbol_dex, _)| symbol_dex.eq_ignore_ascii_case(dex))
            }
        }
    }

    pub(crate) fn symbol_key_is_hidden(&self, key_or_coin: &str) -> bool {
        !Self::key_matches_market_universe(
            &self.exchange_symbols,
            &self.market_universe,
            key_or_coin,
        ) || self.is_ticker_muted(key_or_coin)
    }

    pub(crate) fn exchange_symbol_is_hidden(&self, symbol: &ExchangeSymbol) -> bool {
        !Self::symbol_matches_market_universe(symbol, &self.market_universe)
            || self.exchange_symbol_is_muted(symbol)
    }

    pub(crate) fn market_universe_options(&self) -> Vec<MarketUniverseConfig> {
        let mut dexes = BTreeSet::new();
        for symbol in &self.exchange_symbols {
            if symbol.market_type == MarketType::Perp
                && let Some((dex, _)) = symbol.key.split_once(':')
                && !dex.trim().is_empty()
            {
                dexes.insert(dex.to_ascii_lowercase());
            }
        }

        let mut options = vec![MarketUniverseConfig::All];
        options.extend(dexes.into_iter().map(MarketUniverseConfig::hip3_dex));
        options
    }

    pub(crate) fn market_universe_has_symbols(&self, universe: &MarketUniverseConfig) -> bool {
        match universe {
            MarketUniverseConfig::All => true,
            MarketUniverseConfig::Hip3Dex { dex: _ } => self
                .exchange_symbols
                .iter()
                .any(|symbol| Self::symbol_matches_market_universe(symbol, universe)),
        }
    }

    pub(crate) fn normalize_market_universe_selection(
        &self,
        universe: MarketUniverseConfig,
    ) -> MarketUniverseConfig {
        let universe = universe.normalized();
        if self.exchange_symbols.is_empty() || self.market_universe_has_symbols(&universe) {
            universe
        } else {
            MarketUniverseConfig::All
        }
    }

    pub(crate) fn fallback_unmuted_symbol_key(&self) -> Option<String> {
        for preferred in ["HYPE", "BTC", "ETH"] {
            if !self.symbol_key_is_hidden(preferred) {
                if let Some(symbol) = self.exchange_symbols.iter().find(|symbol| {
                    symbol.key == preferred && self.exchange_symbol_is_orderable(symbol)
                }) {
                    return Some(symbol.key.clone());
                }
                if self.exchange_symbols.is_empty()
                    && matches!(self.market_universe, MarketUniverseConfig::All)
                {
                    return Some(preferred.to_string());
                }
            }
        }

        self.exchange_symbols
            .iter()
            .find(|symbol| {
                symbol.market_type == MarketType::Perp && self.exchange_symbol_is_orderable(symbol)
            })
            .or_else(|| {
                self.exchange_symbols
                    .iter()
                    .find(|symbol| self.exchange_symbol_is_orderable(symbol))
            })
            .map(|symbol| symbol.key.clone())
    }
}
