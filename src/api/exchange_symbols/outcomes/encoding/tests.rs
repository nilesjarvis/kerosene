use super::{outcome_asset_index, outcome_coin_key, outcome_encoding};

#[test]
fn outcome_encoding_matches_hyperliquid_asset_ids() {
    assert_eq!(outcome_encoding(0, 0), 0);
    assert_eq!(outcome_coin_key(1), "#1");
    assert_eq!(outcome_encoding(42, 1), 421);
    assert_eq!(outcome_asset_index(421), 100_000_421);
}
