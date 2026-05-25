use super::*;

#[test]
fn mids_update_tracks_up_and_down_price_flashes() {
    let mut all_mids = HashMap::from([("BTC".to_string(), 10.0), ("ETH".to_string(), 20.0)]);
    let mut updated_at = HashMap::new();
    let mut flashes = HashMap::new();

    apply_mids_update(
        &mut all_mids,
        &mut updated_at,
        &mut flashes,
        HashMap::from([("BTC".to_string(), 11.0), ("ETH".to_string(), 19.0)]),
        42,
        |_| false,
    );

    assert_eq!(flashes.get("BTC").copied(), Some((42, 1)));
    assert_eq!(flashes.get("ETH").copied(), Some((42, -1)));
}

#[test]
fn mids_update_does_not_flash_unchanged_prices() {
    let mut all_mids = HashMap::from([("BTC".to_string(), 10.0)]);
    let mut updated_at = HashMap::new();
    let mut flashes = HashMap::new();

    apply_mids_update(
        &mut all_mids,
        &mut updated_at,
        &mut flashes,
        HashMap::from([("BTC".to_string(), 10.0)]),
        42,
        |_| false,
    );

    assert!(flashes.is_empty());
}
