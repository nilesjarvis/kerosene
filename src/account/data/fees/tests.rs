use serde_json::json;

use super::user_fee_rates_from_value;

#[test]
fn fee_rates_parse_known_string_fields() {
    let rates = user_fee_rates_from_value(&json!({
        "userCrossRate": "0.0003",
        "userAddRate": "0.0001",
        "userSpotCrossRate": "0.0005",
        "userSpotAddRate": "0.0002"
    }));

    assert_eq!(rates.user_cross_rate, "0.0003");
    assert_eq!(rates.user_add_rate, "0.0001");
    assert_eq!(rates.user_spot_cross_rate, "0.0005");
    assert_eq!(rates.user_spot_add_rate, "0.0002");
}

#[test]
fn fee_rates_ignore_missing_or_non_string_fields() {
    let rates = user_fee_rates_from_value(&json!({
        "userCrossRate": 0.0003,
        "userSpotAddRate": "0.0002"
    }));

    assert_eq!(rates.user_cross_rate, "");
    assert_eq!(rates.user_add_rate, "");
    assert_eq!(rates.user_spot_cross_rate, "");
    assert_eq!(rates.user_spot_add_rate, "0.0002");
}

#[test]
fn closing_fee_discount_parses_without_silently_accepting_malformed_terms() {
    let rates = user_fee_rates_from_value(&json!({
        "userCrossRate": "0.001", "activeReferralDiscount": "0.04"
    }));
    assert!((rates.close_taker_rate(false).expect("fee rate") - 0.00096).abs() < 1e-12);
    for value in [json!(0.04), json!("bad"), json!(null)] {
        let rates = user_fee_rates_from_value(&json!({
            "userCrossRate": "0.001", "activeReferralDiscount": value
        }));
        assert!(rates.close_taker_rate(false).is_none());
    }
    let rates = user_fee_rates_from_value(&json!({"userCrossRate": "0.001"}));
    assert_eq!(rates.close_taker_rate(false), Some(0.001));
}
