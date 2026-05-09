use super::*;

#[test]
fn mids_parser_keeps_valid_numeric_prices() {
    let raw = HashMap::from([
        ("BTC".to_string(), "42000.5".to_string()),
        ("ETH".to_string(), "2500".to_string()),
    ]);

    let parsed = parse_mids_response(raw);

    assert_eq!(parsed.get("BTC").copied(), Some(42000.5));
    assert_eq!(parsed.get("ETH").copied(), Some(2500.0));
}

#[test]
fn mids_parser_drops_invalid_price_strings() {
    let raw = HashMap::from([
        ("BTC".to_string(), "not-a-price".to_string()),
        ("ETH".to_string(), "2500".to_string()),
        ("NAN".to_string(), "NaN".to_string()),
        ("INF".to_string(), "inf".to_string()),
        ("ZERO".to_string(), "0".to_string()),
        ("NEG".to_string(), "-1".to_string()),
    ]);

    let parsed = parse_mids_response(raw);

    assert!(!parsed.contains_key("BTC"));
    assert!(!parsed.contains_key("NAN"));
    assert!(!parsed.contains_key("INF"));
    assert!(!parsed.contains_key("ZERO"));
    assert!(!parsed.contains_key("NEG"));
    assert_eq!(parsed.get("ETH").copied(), Some(2500.0));
}
