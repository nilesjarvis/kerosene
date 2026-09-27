use super::*;
use serde_json::json;

#[test]
fn listings_perps_include_native_hip3_and_inactive_identities() {
    let markets = parse_perps(json!([
        {"universe": [{"name":"BTC"}, {"name":"OLD", "isDelisted":true}]},
        {"universe": [{"name":"xyz:BTC"}]}
    ]))
    .expect("valid perps");
    assert_eq!(markets.len(), 3);
    assert!(!markets[1].active);
    assert_ne!(markets[0].id, markets[2].id);
    assert_eq!(markets[2].label, "BTC");
}

#[test]
fn listings_perps_reject_incomplete_or_ambiguous_metadata() {
    for value in [
        json!({"error":"offline"}),
        json!([]),
        json!([{"universe":[]}]),
        json!([{"universe":[{"name":"BTC"}]}, {}]),
        json!([{"universe":[{"name":"BTC"}, {"name":"BTC"}]}]),
        json!([{"universe":[{"name":"BTC", "isDelisted":"false"}]}]),
        json!([{"universe":[{"name":"BTC"}]}, {"universe":[{"name":"NEW"}]}]),
    ] {
        assert!(parse_perps(value).is_err());
    }
}

#[test]
fn listings_spot_tracks_pairs_not_unpaired_tokens_and_preserves_quote() {
    let markets = parse_spot(json!({
        "tokens":[{"name":"USDC","szDecimals":8,"index":0}, {"name":"TEST","szDecimals":2,"index":7}, {"name":"OTHER","szDecimals":2,"index":9}],
        "universe":[{"name":"@3","tokens":[7,0],"index":3}, {"name":"@4","tokens":[7,9],"index":4}]
    })).expect("valid spot pairs");
    assert_eq!(markets.len(), 2);
    assert_eq!(markets[0].id, "spot:10003");
    assert_eq!(markets[0].label, "TEST/USDC");
    assert_eq!(markets[1].label, "TEST/OTHER");
    assert_eq!(markets[1].key, "@4");
}

#[test]
fn listings_spot_rejects_invalid_references() {
    assert!(parse_spot(json!({"tokens":[{"name":"USDC","szDecimals":8,"index":0}], "universe":[{"name":"@1","tokens":[99,0],"index":1}]})).is_err());
}
