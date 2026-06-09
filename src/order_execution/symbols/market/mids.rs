use super::live_mids::resolve_live_mid_from_candidates;
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;

// ---------------------------------------------------------------------------
// Market Mid Prices
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn mid_candidates_for_symbol(&self, symbol: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut push_unique = |value: String| {
            if !value.is_empty() && !out.contains(&value) {
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
            let message = format!(
                "Order price {} is {:.1}% away from {} reference {}. \
                Press Mid or update the price before submitting. Tried mids: {}",
                format_price(price),
                distance * 100.0,
                symbol,
                format_price(reference),
                candidates
            );
            Err(message)
        } else {
            Ok(())
        }
    }
}
