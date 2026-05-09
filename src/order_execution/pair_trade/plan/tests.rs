use super::*;

fn candidates(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn parse_pair_notional_accepts_trimmed_positive_finite_values() {
    assert_eq!(parse_pair_notional(" 1250.50 "), Some(1250.50));
}

#[test]
fn parse_pair_notional_rejects_zero_negative_nan_and_infinite_values() {
    assert_eq!(parse_pair_notional("0"), None);
    assert_eq!(parse_pair_notional("-1"), None);
    assert_eq!(parse_pair_notional("NaN"), None);
    assert_eq!(parse_pair_notional("inf"), None);
    assert_eq!(parse_pair_notional("notional"), None);
}

#[test]
fn pair_leg_sides_invert_between_legs() {
    assert_eq!(pair_leg_sides(true), (true, false));
    assert_eq!(pair_leg_sides(false), (false, true));
}

#[test]
fn pair_direction_label_matches_trade_mode() {
    assert_eq!(
        pair_direction_label("BTC", "ETH", true),
        "Long BTC / Short ETH"
    );
    assert_eq!(
        pair_direction_label("BTC", "ETH", false),
        "Short BTC / Long ETH"
    );
}

#[test]
fn missing_pair_mid_status_reports_only_missing_legs() {
    assert_eq!(
        missing_pair_mid_status(
            "BTC",
            "ETH",
            0.0,
            100.0,
            &candidates(&["BTC", "ubtc:BTC"]),
            &candidates(&["ETH"])
        ),
        Some("Missing mid prices for pair legs: A=BTC (tried BTC, ubtc:BTC)".to_string())
    );
    assert_eq!(
        missing_pair_mid_status(
            "BTC",
            "ETH",
            0.0,
            -1.0,
            &candidates(&["BTC"]),
            &candidates(&["ETH", "ueth:ETH"])
        ),
        Some(
            "Missing mid prices for pair legs: A=BTC (tried BTC); B=ETH (tried ETH, ueth:ETH)"
                .to_string()
        )
    );
    assert_eq!(
        missing_pair_mid_status(
            "BTC",
            "ETH",
            100.0,
            200.0,
            &candidates(&["BTC"]),
            &candidates(&["ETH"])
        ),
        None
    );
}

#[test]
fn missing_pair_mid_status_rejects_non_finite_mids() {
    assert_eq!(
        missing_pair_mid_status(
            "BTC",
            "ETH",
            f64::NAN,
            f64::INFINITY,
            &candidates(&["BTC"]),
            &candidates(&["ETH"])
        ),
        Some("Missing mid prices for pair legs: A=BTC (tried BTC); B=ETH (tried ETH)".to_string())
    );
}
