use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

pub(crate) const LIVE_MID_MAX_AGE_MS: u64 = 15_000;

fn valid_mid_price(price: f64) -> bool {
    price.is_finite() && price > 0.0
}

fn live_mid_is_fresh(updated_at_ms: u64, now_ms: u64) -> bool {
    now_ms
        .checked_sub(updated_at_ms)
        .is_some_and(|age_ms| age_ms <= LIVE_MID_MAX_AGE_MS)
}

fn resolve_live_mid_from_candidates(
    candidates: &[String],
    all_mids: &HashMap<String, f64>,
    all_mids_updated_at_ms: &HashMap<String, u64>,
    now_ms: u64,
) -> Option<f64> {
    for candidate in candidates {
        if let (Some(price), Some(updated_at_ms)) = (
            all_mids.get(candidate).copied(),
            all_mids_updated_at_ms.get(candidate).copied(),
        ) && valid_mid_price(price)
            && live_mid_is_fresh(updated_at_ms, now_ms)
        {
            return Some(price);
        }
    }
    None
}

impl TradingTerminal {
    pub(crate) fn resolve_exchange_symbol_by_key_or_ticker(
        &self,
        key_or_ticker: &str,
    ) -> Option<&ExchangeSymbol> {
        self.exchange_symbols
            .iter()
            .find(|s| s.key == key_or_ticker)
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
        if !symbol.is_user_selectable_market() {
            return Err(format!(
                "{} is not a tradable market",
                Self::exchange_symbol_display_name(symbol)
            ));
        }
        if symbol.market_type == MarketType::Outcome && symbol.outcome.is_none() {
            return Err(format!(
                "{} outcome metadata is incomplete",
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
                && self.market_type_for_symbol(coin).is_none())
    }

    pub(crate) fn mid_candidates_for_symbol(&self, symbol: &str) -> Vec<String> {
        let _theme = self.theme();
        let mut out = Vec::new();
        let mut push_unique = |value: String| {
            if !value.is_empty() && !out.iter().any(|v| v == &value) {
                out.push(value);
            }
        };

        push_unique(symbol.to_string());
        if let Some(encoding) = symbol.strip_prefix('+') {
            push_unique(format!("#{encoding}"));
        }
        if let Some((dex, suffix)) = symbol.split_once(':') {
            if let Some(stripped) = suffix.strip_prefix('U') {
                push_unique(format!("{dex}:{stripped}"));
            }
        } else if let Some(stripped) = symbol.strip_prefix('U') {
            push_unique(stripped.to_string());
        }

        if let Some(sym) = self.exchange_symbols.iter().find(|s| s.key == symbol) {
            push_unique(sym.key.clone());
            if let Some((dex, ticker)) = sym.key.split_once(':') {
                if let Some(stripped) = ticker.strip_prefix('U') {
                    push_unique(format!("{dex}:{stripped}"));
                }
            } else {
                push_unique(sym.ticker.clone());
                push_unique(format!("U{}", sym.ticker));
            }
        }

        out
    }

    pub(crate) fn resolve_mid_for_symbol(&self, symbol: &str) -> Option<f64> {
        let _theme = self.theme();
        resolve_live_mid_from_candidates(
            &self.mid_candidates_for_symbol(symbol),
            &self.all_mids,
            &self.all_mids_updated_at_ms,
            Self::now_ms(),
        )
    }

    pub(crate) fn refresh_order_price_for_symbol(&mut self, symbol: &str) {
        if matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc) {
            if let Some(mid) = self.resolve_mid_for_symbol(symbol) {
                self.order_price = format_price(mid);
            } else {
                self.order_price.clear();
            }
        }
    }

    pub(crate) fn validate_order_price_band(&self, symbol: &str, price: f64) -> Result<(), String> {
        let Some(reference) = self.resolve_mid_for_symbol(symbol) else {
            return Err(format!(
                "No mid price for {} (tried {})",
                symbol,
                self.mid_candidates_for_symbol(symbol).join(", ")
            ));
        };
        if reference <= 0.0 || price <= 0.0 {
            return Ok(());
        }

        let distance = ((price / reference) - 1.0).abs();
        if distance > 0.95 {
            let candidates = self.mid_candidates_for_symbol(symbol).join(", ");
            Err(format!(
                "Order price {} is {:.1}% away from {} reference {}. Press Mid or update the price before submitting. Tried mids: {}",
                format_price(price),
                distance * 100.0,
                symbol,
                format_price(reference),
                candidates
            ))
        } else {
            Ok(())
        }
    }

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
