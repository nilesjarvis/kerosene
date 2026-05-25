use super::*;

#[test]
fn mids_update_inserts_new_prices_without_flashes() {
    let mut all_mids = HashMap::new();
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

    assert_eq!(all_mids.get("BTC").copied(), Some(10.0));
    assert_eq!(updated_at.get("BTC").copied(), Some(42));
    assert!(flashes.is_empty());
}
