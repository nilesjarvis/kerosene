use crate::api::{MarketType, USDC_TOKEN_INDEX};
use crate::app_state::TradingTerminal;

pub(crate) const OUTCOME_MIN_PRICE: f64 = 0.001;
pub(crate) const OUTCOME_MAX_PRICE: f64 = 0.999;

impl TradingTerminal {
    pub(crate) fn outcome_read_only_status(&mut self, action: &str) {
        self.order_status = Some((
            format!(
                "Outcome {action} is not available from this control; use the main order ticket"
            ),
            true,
        ));
    }

    pub(crate) fn outcome_balance_coin_to_trade_coin(coin: &str) -> Option<String> {
        coin.strip_prefix('+')
            .map(|encoding| format!("#{encoding}"))
    }

    pub(crate) fn outcome_trade_coin_for_balance_coin(&self, coin: &str) -> Option<String> {
        let trade_coin = Self::outcome_balance_coin_to_trade_coin(coin)?;
        self.exchange_symbols
            .iter()
            .any(|symbol| {
                symbol.key == trade_coin
                    && symbol.market_type == crate::api::MarketType::Outcome
                    && symbol.is_user_selectable_market()
            })
            .then_some(trade_coin)
    }

    pub(crate) fn outcome_quote_symbol_for_coin(&self, coin: &str) -> String {
        self.exchange_symbols
            .iter()
            .find(|symbol| symbol.key == coin && symbol.market_type == MarketType::Outcome)
            .and_then(|symbol| symbol.outcome.as_ref())
            .map(|info| info.quote_symbol.clone())
            .unwrap_or_else(|| "USDC".to_string())
    }

    pub(crate) fn outcome_quote_token_index_for_coin(&self, coin: &str) -> u32 {
        self.exchange_symbols
            .iter()
            .find(|symbol| symbol.key == coin && symbol.market_type == MarketType::Outcome)
            .and_then(|symbol| symbol.outcome.as_ref())
            .and_then(|info| info.quote_token_index)
            .unwrap_or(USDC_TOKEN_INDEX)
    }

    pub(crate) fn display_coin_for_spot_balance(&self, coin: &str) -> String {
        if let Some(symbol) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == coin && symbol.market_type == MarketType::Spot)
        {
            return Self::exchange_symbol_display_name(symbol);
        }

        self.outcome_trade_coin_for_balance_coin(coin)
            .map(|trade_coin| format!("{} ({coin})", self.display_name_for_symbol(&trade_coin)))
            .unwrap_or_else(|| coin.to_string())
    }

    pub(crate) fn validate_outcome_order_price(price: f64) -> Result<(), String> {
        if price.is_finite() && (OUTCOME_MIN_PRICE..=OUTCOME_MAX_PRICE).contains(&price) {
            Ok(())
        } else {
            Err(format!(
                "Outcome prices must be between {OUTCOME_MIN_PRICE:.3} and {OUTCOME_MAX_PRICE:.3}"
            ))
        }
    }

    pub(crate) fn clamp_outcome_market_price(price: f64) -> f64 {
        price.clamp(OUTCOME_MIN_PRICE, OUTCOME_MAX_PRICE)
    }

    pub(crate) fn validate_outcome_contract_size(&self, qty: f64) -> Result<(), String> {
        if qty.fract().abs() <= 1e-9 {
            Ok(())
        } else {
            Err("Outcome orders require whole-contract sizes".to_string())
        }
    }

    pub(crate) fn sanitize_outcome_quantity_input(input: &str) -> String {
        let mut out = String::new();
        for ch in input.chars() {
            if ch.is_ascii_digit() {
                out.push(ch);
            } else if ch == '.' || ch == ',' {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
