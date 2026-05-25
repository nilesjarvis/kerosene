use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, parse_number, positive_finite_value};
use crate::signing::round_price;

// ---------------------------------------------------------------------------
// TWAP Price Range
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn effective_twap_price_range(
        &mut self,
        sz_decimals: u32,
        is_spot: bool,
    ) -> Option<(f64, f64)> {
        let parsed_min = parse_positive_price(&self.twap_form.min_price);
        let parsed_max = parse_positive_price(&self.twap_form.max_price);
        let mut min_price = parsed_min;
        let mut max_price = parsed_max;

        if min_price.is_none() || max_price.is_none() {
            let mid = self
                .resolve_mid_for_symbol(&self.active_symbol)
                .and_then(positive_finite_value)?;
            let width = self.market_slippage_fraction().max(0.001);
            if min_price.is_none() {
                min_price = Some(round_price(mid * (1.0 - width), sz_decimals, is_spot));
            }
            if max_price.is_none() {
                max_price = Some(round_price(mid * (1.0 + width), sz_decimals, is_spot));
            }
        }

        let min_price = positive_finite_value(min_price?)?;
        let max_price = positive_finite_value(max_price?)?;
        if max_price <= min_price {
            return None;
        }
        self.twap_form.min_price = format_price(min_price);
        self.twap_form.max_price = format_price(max_price);
        Some((min_price, max_price))
    }
}

pub(super) fn parse_positive_price(value: &str) -> Option<f64> {
    parse_number(value).and_then(positive_finite_value)
}
