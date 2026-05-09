use super::parse_outcome_description;

#[test]
fn outcome_description_parser_extracts_known_fields() {
    let parsed = parse_outcome_description(
        "class:priceBinary|underlying:BTC|expiry:20260503-0600|targetPrice:78213|period:1d",
    );

    assert_eq!(parsed.get("class").map(String::as_str), Some("priceBinary"));
    assert_eq!(parsed.get("underlying").map(String::as_str), Some("BTC"));
    assert_eq!(parsed.get("targetPrice").map(String::as_str), Some("78213"));
}
