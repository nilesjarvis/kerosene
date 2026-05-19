use crate::account::{AccountData, ClearinghouseState};
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::MarketUniverseConfig;
use std::collections::{BTreeSet, HashSet};

#[cfg(test)]
mod tests;

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

    pub(crate) fn visible_clearinghouse_state<'a>(
        &self,
        data: &'a AccountData,
    ) -> &'a ClearinghouseState {
        self.market_universe
            .selected_hip3_dex()
            .and_then(|dex| data.clearinghouses_by_dex.get(dex))
            .unwrap_or(&data.clearinghouse)
    }

    pub(crate) fn market_universe_includes_spot(&self) -> bool {
        self.market_universe.selected_hip3_dex().is_none()
    }

    pub(crate) fn account_view_includes_spot_balances(&self, data: &AccountData) -> bool {
        self.market_universe_includes_spot() || data.is_portfolio_margin()
    }

    pub(crate) fn account_spot_balance_is_hidden(&self, data: &AccountData, coin: &str) -> bool {
        let outcome_trade_coin = self.outcome_trade_coin_for_balance_coin(coin);
        if let Some(trade_coin) = Self::outcome_balance_coin_to_trade_coin(coin)
            && self.exchange_symbols.iter().any(|symbol| {
                symbol.key == trade_coin
                    && symbol.market_type == MarketType::Outcome
                    && !symbol.is_user_selectable_market()
            })
        {
            return true;
        }

        if data.is_portfolio_margin() {
            self.is_ticker_muted(coin)
                || outcome_trade_coin
                    .as_deref()
                    .is_some_and(|trade_coin| self.is_ticker_muted(trade_coin))
        } else {
            outcome_trade_coin
                .as_deref()
                .map(|trade_coin| self.symbol_key_is_hidden(trade_coin))
                .unwrap_or_else(|| self.symbol_key_is_hidden(coin))
        }
    }

    pub(crate) fn visible_collateral_token(&self) -> Option<u32> {
        self.market_universe
            .selected_hip3_dex()
            .and_then(|selected_dex| {
                self.exchange_symbols.iter().find_map(|symbol| {
                    if symbol.market_type == MarketType::Perp
                        && symbol.key.split_once(':').is_some_and(|(symbol_dex, _)| {
                            symbol_dex.eq_ignore_ascii_case(selected_dex)
                        })
                    {
                        symbol.collateral_token
                    } else {
                        None
                    }
                })
            })
            .or(Some(0).filter(|_| self.market_universe.selected_hip3_dex().is_none()))
    }

    pub(crate) fn visible_available_margin_usdc(&self, data: &AccountData) -> Option<f64> {
        if data.is_portfolio_margin() {
            return data.available_margin_usdc();
        }

        if data.uses_shared_account_balance() {
            return self
                .visible_collateral_token()
                .and_then(|token| data.available_margin_for_token(token));
        }

        if self.market_universe.selected_hip3_dex().is_some() {
            return self
                .visible_clearinghouse_state(data)
                .withdrawable
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite());
        }

        data.available_margin_usdc()
    }

    pub(crate) fn sorted_muted_tickers(&self) -> Vec<String> {
        let mut tickers: Vec<String> = self.muted_tickers.iter().cloned().collect();
        tickers.sort();
        tickers
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
