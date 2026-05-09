use super::super::UserFeeRates;
use serde_json::Value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Fee Response Parsing
// ---------------------------------------------------------------------------

pub(super) fn user_fee_rates_from_value(raw: &Value) -> UserFeeRates {
    UserFeeRates {
        user_cross_rate: raw
            .get("userCrossRate")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        user_add_rate: raw
            .get("userAddRate")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        user_spot_cross_rate: raw
            .get("userSpotCrossRate")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        user_spot_add_rate: raw
            .get("userSpotAddRate")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    }
}
