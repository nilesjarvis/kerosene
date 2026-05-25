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
    ) -> Option<(f64, f64)> {
        let _theme = self.theme();
        let rates = &self.account_data.as_ref()?.fee_rates;
        let rate = rates.rate_for(is_limit, is_spot)?;
        let notional = price * qty;
        let notional = positive_finite_value(notional)?;
        Some((notional * rate, rate))
    }
}
