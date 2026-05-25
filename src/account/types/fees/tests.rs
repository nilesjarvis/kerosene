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
