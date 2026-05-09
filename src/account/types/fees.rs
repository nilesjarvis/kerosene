use serde::Deserialize;

// ---------------------------------------------------------------------------
// Fee Rates
// ---------------------------------------------------------------------------

/// User's personalized fee rates (from the `userFees` endpoint).
/// Rates already include VIP tier discounts, referral discounts,
/// and staking discounts.
#[derive(Debug, Clone, Deserialize, Default)]
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
}

impl UserFeeRates {
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
mod tests {
    use super::UserFeeRates;

    #[test]
    fn fee_rate_for_rejects_malformed_negative_or_nonfinite_rates() {
        let rates = UserFeeRates {
            user_cross_rate: "bad".to_string(),
            user_add_rate: "NaN".to_string(),
            user_spot_cross_rate: "-0.1".to_string(),
            user_spot_add_rate: "0.0002".to_string(),
        };

        assert_eq!(rates.rate_for(false, false), None);
        assert_eq!(rates.rate_for(true, false), None);
        assert_eq!(rates.rate_for(false, true), None);
        assert_eq!(rates.rate_for(true, true), Some(0.0002));
    }
}
