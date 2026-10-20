use super::UserFeeRates;

#[test]
fn fee_rate_for_rejects_malformed_negative_or_nonfinite_rates() {
    let rates = UserFeeRates {
        user_cross_rate: "bad".to_string(),
        user_add_rate: "NaN".to_string(),
        user_spot_cross_rate: "-0.1".to_string(),
        user_spot_add_rate: "0.0002".to_string(),
        active_referral_discount: None,
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
        active_referral_discount: Some("referral-discount-secret".into()),
    };

    let rendered = format!("{rates:?}");

    assert!(rendered.contains("UserFeeRates"));
    assert!(rendered.contains("<redacted>"));
    for secret in [
        "cross-rate-secret",
        "add-rate-secret",
        "spot-cross-rate-secret",
        "spot-add-rate-secret",
        "referral-discount-secret",
    ] {
        assert!(!rendered.contains(secret), "fee rate Debug leaked {secret}");
    }
}

#[test]
fn closing_taker_rates_include_referrals_and_reject_unknown_discounts() {
    let mut rates = UserFeeRates {
        user_cross_rate: "0.0004".into(),
        user_spot_cross_rate: "0.0005".into(),
        active_referral_discount: Some("0.04".into()),
        ..Default::default()
    };
    assert!((rates.close_taker_rate(false).expect("perp rate") - 0.000384).abs() < 1e-12);
    assert!((rates.close_taker_rate(true).expect("spot rate") - 0.00048).abs() < 1e-12);
    for discount in ["bad", "NaN", "-0.1", "1.1", ""] {
        rates.active_referral_discount = Some(discount.into());
        assert!(rates.close_taker_rate(false).is_none());
    }
    rates.active_referral_discount = None;
    assert_eq!(rates.close_taker_rate(false), Some(0.0004));
}
