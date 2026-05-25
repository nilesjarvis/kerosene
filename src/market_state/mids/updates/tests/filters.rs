use super::*;

#[test]
fn mids_update_skips_muted_symbols() {
    let mut all_mids = HashMap::from([("BTC".to_string(), 10.0)]);
    let mut updated_at = HashMap::from([("BTC".to_string(), 1)]);
    let mut flashes = HashMap::new();

    apply_mids_update(
        &mut all_mids,
        &mut updated_at,
        &mut flashes,
        HashMap::from([("BTC".to_string(), 11.0)]),
        42,
        |symbol| symbol == "BTC",
    );

    assert_eq!(all_mids.get("BTC").copied(), Some(10.0));
    assert_eq!(updated_at.get("BTC").copied(), Some(1));
    assert!(flashes.is_empty());
}

#[test]
fn mids_update_can_retain_display_denomination_rate_when_hidden_elsewhere() {
    let mut all_mids = HashMap::new();
    let mut updated_at = HashMap::new();
    let mut flashes = HashMap::new();

    apply_mids_update(
        &mut all_mids,
        &mut updated_at,
        &mut flashes,
        HashMap::from([
            ("xyz:EUR".to_string(), 1.2),
            ("xyz:NVDA".to_string(), 100.0),
        ]),
        42,
        |symbol| symbol != "xyz:EUR",
    );

    assert_eq!(all_mids.get("xyz:EUR").copied(), Some(1.2));
    assert_eq!(updated_at.get("xyz:EUR").copied(), Some(42));
    assert!(!all_mids.contains_key("xyz:NVDA"));
}

#[test]
fn mids_update_skips_nonpositive_or_nonfinite_prices() {
    let mut all_mids = HashMap::from([("BTC".to_string(), 10.0)]);
    let mut updated_at = HashMap::from([("BTC".to_string(), 1)]);
    let mut flashes = HashMap::new();

    apply_mids_update(
        &mut all_mids,
        &mut updated_at,
        &mut flashes,
        HashMap::from([
            ("BTC".to_string(), f64::NAN),
            ("ZERO".to_string(), 0.0),
            ("NEG".to_string(), -1.0),
            ("INF".to_string(), f64::INFINITY),
        ]),
        42,
        |_| false,
    );

    assert_eq!(all_mids, HashMap::from([("BTC".to_string(), 10.0)]));
    assert_eq!(updated_at, HashMap::from([("BTC".to_string(), 1)]));
    assert!(flashes.is_empty());
}
