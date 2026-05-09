use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Muted Ticker Matching
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn normalize_muted_ticker_input(input: &str) -> Option<String> {
        let trimmed = input.trim().trim_matches(|c| c == '{' || c == '}');
        let trimmed = trimmed.strip_prefix('$').unwrap_or(trimmed).trim();
        (!trimmed.is_empty()).then(|| trimmed.to_ascii_uppercase())
    }

    pub(crate) fn push_unique_ticker_candidate(candidates: &mut Vec<String>, candidate: String) {
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    pub(crate) fn muted_ticker_candidates_for_key(key_or_coin: &str) -> Vec<String> {
        let mut candidates = Vec::new();
        let normalized = key_or_coin.trim().to_ascii_uppercase();
        if normalized.is_empty() {
            return candidates;
        }

        Self::push_unique_ticker_candidate(&mut candidates, normalized.clone());

        if let Some((_dex, coin)) = normalized.split_once(':') {
            Self::push_unique_ticker_candidate(&mut candidates, coin.to_string());
            if let Some(stripped) = coin.strip_prefix('U')
                && !stripped.is_empty()
            {
                Self::push_unique_ticker_candidate(&mut candidates, stripped.to_string());
            }
        } else if let Some(stripped) = normalized.strip_prefix('U')
            && !stripped.is_empty()
        {
            Self::push_unique_ticker_candidate(&mut candidates, stripped.to_string());
        }

        candidates
    }

    pub(crate) fn muted_ticker_candidates_for_exchange_symbol(
        symbol: &ExchangeSymbol,
    ) -> Vec<String> {
        let mut candidates = Self::muted_ticker_candidates_for_key(&symbol.key);

        for candidate in Self::muted_ticker_candidates_for_key(&symbol.ticker) {
            Self::push_unique_ticker_candidate(&mut candidates, candidate);
        }

        if let Some(display) = symbol.display_name.as_deref() {
            for part in [display, display.split('/').next().unwrap_or(display)] {
                for candidate in Self::muted_ticker_candidates_for_key(part) {
                    Self::push_unique_ticker_candidate(&mut candidates, candidate);
                }
            }
        }

        candidates
    }

    pub(crate) fn key_matches_muted_tickers(
        exchange_symbols: &[ExchangeSymbol],
        muted_tickers: &HashSet<String>,
        key_or_coin: &str,
    ) -> bool {
        let normalized = match Self::normalize_muted_ticker_input(key_or_coin) {
            Some(value) => value,
            None => return false,
        };

        Self::muted_ticker_candidates_for_key(&normalized)
            .iter()
            .any(|candidate| muted_tickers.contains(candidate))
            || exchange_symbols
                .iter()
                .find(|symbol| {
                    symbol.key.eq_ignore_ascii_case(key_or_coin)
                        || symbol.ticker.eq_ignore_ascii_case(key_or_coin)
                })
                .is_some_and(|symbol| {
                    Self::exchange_symbol_matches_muted_tickers(symbol, muted_tickers)
                })
    }

    pub(crate) fn exchange_symbol_matches_muted_tickers(
        symbol: &ExchangeSymbol,
        muted_tickers: &HashSet<String>,
    ) -> bool {
        Self::muted_ticker_candidates_for_exchange_symbol(symbol)
            .iter()
            .any(|candidate| muted_tickers.contains(candidate))
    }

    pub(crate) fn is_ticker_muted(&self, key_or_coin: &str) -> bool {
        Self::key_matches_muted_tickers(&self.exchange_symbols, &self.muted_tickers, key_or_coin)
    }

    pub(crate) fn exchange_symbol_is_muted(&self, symbol: &ExchangeSymbol) -> bool {
        Self::exchange_symbol_matches_muted_tickers(symbol, &self.muted_tickers)
    }

    pub(crate) fn sorted_muted_tickers(&self) -> Vec<String> {
        let mut tickers: Vec<String> = self.muted_tickers.iter().cloned().collect();
        tickers.sort();
        tickers
    }

    pub(crate) fn fallback_unmuted_symbol_key(&self) -> Option<String> {
        for preferred in ["HYPE", "BTC", "ETH"] {
            if !self.is_ticker_muted(preferred) {
                if let Some(symbol) = self.exchange_symbols.iter().find(|symbol| {
                    symbol.key == preferred && !self.exchange_symbol_is_muted(symbol)
                }) {
                    return Some(symbol.key.clone());
                }
                if self.exchange_symbols.is_empty() {
                    return Some(preferred.to_string());
                }
            }
        }

        self.exchange_symbols
            .iter()
            .find(|symbol| {
                symbol.market_type == MarketType::Perp && !self.exchange_symbol_is_muted(symbol)
            })
            .or_else(|| {
                self.exchange_symbols
                    .iter()
                    .find(|symbol| !self.exchange_symbol_is_muted(symbol))
            })
            .map(|symbol| symbol.key.clone())
    }
}
