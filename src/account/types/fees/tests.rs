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

#[test]
fn fee_rates_debug_redacts_personalized_rates() {
    let rates = UserFeeRates {
        user_cross_rate: "cross-rate-secret".to_string(),
        user_add_rate: "add-rate-secret".to_string(),
        user_spot_cross_rate: "spot-cross-rate-secret".to_string(),
        user_spot_add_rate: "spot-add-rate-secret".to_string(),
    };

    let rendered = format!("{rates:?}");

    assert!(rendered.contains("UserFeeRates"));
    assert!(rendered.contains("<redacted>"));
    for secret in [
        "cross-rate-secret",
        "add-rate-secret",
        "spot-cross-rate-secret",
        "spot-add-rate-secret",
    ] {
        assert!(!rendered.contains(secret), "fee rate Debug leaked {secret}");
    }
}
