use super::*;
use serde_json::json;

fn instrument(id: u32, symbol: &str) -> Value {
    json!([
        id, symbol, id, "10", true, false, false, false, false, null, null, null, "15000", "800",
        1000, "10000000", "0", "0", "0", "0", "0", "0", "0"
    ])
}

fn catalog(instruments: Value) -> Value {
    json!({
        "schemaVersion": 1,
        "indexedThroughBlock": 1234,
        "tradingPaused": false,
        "instruments": instruments,
        "intakeControls": [1, "1000", []],
        "maximumLeverage": 0,
        "minimumLeverage": 0
    })
}

fn parse(value: &Value) -> Result<PapertradeMarkets, String> {
    parse_markets(&serde_json::to_vec(value).expect("fixture JSON"))
}

#[test]
fn papertrade_catalog_uses_instrument_identity_and_live_tuple_metadata() {
    let mut retired = instrument(4, "BTC");
    retired[7] = json!(true);
    let data = parse(&catalog(json!([
        instrument(1, "ETH"),
        retired,
        instrument(0, "BTC")
    ])))
    .expect("catalog");
    assert_eq!(data.indexed_through_block, 1234);
    assert_eq!(data.markets.len(), 3);
    assert_eq!(data.markets[0].id, PapertradeMarketId(0));
    assert_eq!(data.markets[0].max_leverage, 1000);
    assert_eq!(data.markets[1].id, PapertradeMarketId(4));
    assert_eq!(data.markets[1].status, "Retired");
    assert!(data.markets[2].matches_query("eth"));
    assert!(!data.markets[2].matches_query("btc"));
}

#[test]
fn papertrade_catalog_rejects_unknown_schema_duplicates_and_invalid_rows() {
    let valid = catalog(json!([instrument(0, "BTC")]));
    let mut unknown = valid.clone();
    unknown["schemaVersion"] = json!(2);
    assert!(parse(&unknown).is_err());
    assert!(
        parse(&catalog(json!([
            instrument(0, "BTC"),
            instrument(0, "ETH")
        ])))
        .is_err()
    );
    for (index, value) in [
        (0, json!(-1)),
        (1, json!("BTC\n")),
        (4, json!("true")),
        (14, json!(1.5)),
    ] {
        let mut malformed = valid.clone();
        malformed["instruments"][0][index] = value;
        assert!(parse(&malformed).is_err());
    }
    assert!(parse(&catalog(json!([[0, "BTC"]]))).is_err());
    assert!(parse_markets(b"not json").is_err());
    assert!(
        parse(&catalog(json!([])))
            .expect("empty catalog")
            .markets
            .is_empty()
    );
}

#[test]
fn papertrade_catalog_preserves_market_restrictions() {
    for (index, value, expected) in [
        (4, false, "Inactive"),
        (5, true, "Paused"),
        (6, true, "Retired"),
        (8, true, "Close only"),
    ] {
        let mut row = instrument(0, "BTC");
        row[index] = json!(value);
        assert_eq!(
            parse(&catalog(json!([row]))).expect("catalog").markets[0].status,
            expected
        );
    }
    let mut paused = catalog(json!([instrument(0, "BTC")]));
    paused["tradingPaused"] = json!(true);
    assert_eq!(parse(&paused).expect("catalog").markets[0].status, "Paused");
}
