use crate::app_state::TradingTerminal;
use crate::helpers::positive_finite_value;

impl TradingTerminal {
    /// Estimate the fee for a trade. Returns `(fee_amount, fee_rate)` or None
    /// if fee data isn't available or inputs are invalid.
    pub(crate) fn estimate_fee(
        &self,
        price: f64,
        qty: f64,
        is_limit: bool,
        is_spot: bool,
        is_buy: bool,
    ) -> Option<(f64, f64)> {
        let rates = &self.connected_order_account_snapshot()?.1.fee_rates;
        let rate = rates.rate_for(is_limit, is_spot)? + self.builder_fee_rate(is_spot, is_buy)?;
        let notional = positive_finite_value(price)? * positive_finite_value(qty)?;
        let notional = positive_finite_value(notional)?;
        Some((notional * rate, rate))
    }

    /// Builder fees apply to both perpetual sides and only the quote-receiving
    /// sell side of spot-like markets (including outcome contracts).
    pub(crate) fn builder_fee_rate(&self, is_spot_like: bool, is_buy: bool) -> Option<f64> {
        let builder = self.builder_codes.config.resolve().ok()?;
        Some(builder.map_or(0.0, |builder| {
            if is_spot_like && is_buy {
                0.0
            } else {
                f64::from(builder.fee_tenths_bps()) / 100_000.0
            }
        }))
    }
}

#[cfg(test)]
mod tests;
