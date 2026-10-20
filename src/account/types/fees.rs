use serde::Deserialize;
use std::fmt;

// ---------------------------------------------------------------------------
// Fee Rates
// ---------------------------------------------------------------------------

/// User's personalized fee rates (from the `userFees` endpoint).
/// Rates include VIP tier and staking discounts. Referral discounts are supplied
/// separately by the API.
#[derive(Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserFeeRates {
    /// Perp taker (cross) fee rate, e.g. "0.000315" = 0.0315%.
    #[serde(default)]
    pub user_cross_rate: String,
    /// Perp maker (add) fee rate, e.g. "0.000105" = 0.0105%.
    #[serde(default)]
    pub user_add_rate: String,
    /// Spot taker (cross) fee rate, e.g. "0.00049" = 0.049%.
    #[serde(default)]
    pub user_spot_cross_rate: String,
    /// Spot maker (add) fee rate, e.g. "0.00028" = 0.028%.
    #[serde(default)]
    pub user_spot_add_rate: String,
    #[serde(default)]
    pub active_referral_discount: Option<String>,
}

impl fmt::Debug for UserFeeRates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserFeeRates")
            .field("user_cross_rate", &"<redacted>")
            .field("user_add_rate", &"<redacted>")
            .field("user_spot_cross_rate", &"<redacted>")
            .field("user_spot_add_rate", &"<redacted>")
            .field("active_referral_discount", &"<redacted>")
            .finish()
    }
}

impl UserFeeRates {
    /// Taker rate before market-specific multipliers and builder fees.
    pub(crate) fn close_taker_rate(&self, is_spot: bool) -> Option<f64> {
        let discount = self
            .active_referral_discount
            .as_deref()
            .unwrap_or("0")
            .trim()
            .parse::<f64>()
            .ok()?;
        if !discount.is_finite() || !(0.0..=1.0).contains(&discount) {
            return None;
        }
        Some(self.rate_for(false, is_spot)? * (1.0 - discount))
    }

    /// Get the appropriate fee rate for a given order type.
    /// Returns the rate as a fraction (e.g. 0.000315 for 0.0315%).
    pub fn rate_for(&self, is_limit: bool, is_spot: bool) -> Option<f64> {
        let fee_rate = if is_spot {
            if is_limit {
                &self.user_spot_add_rate
            } else {
                &self.user_spot_cross_rate
            }
        } else if is_limit {
            &self.user_add_rate
        } else {
            &self.user_cross_rate
        };
        let rate = fee_rate.trim().parse::<f64>().ok()?;
        (rate.is_finite() && rate >= 0.0).then_some(rate)
    }
}

#[cfg(test)]
mod tests;
