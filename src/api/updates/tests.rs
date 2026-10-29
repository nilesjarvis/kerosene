use super::*;
use serde_json::json;

#[tokio::test]
#[ignore = "requires public Hyperliquid network access"]
async fn updates_live_public_snapshot_parses() {
    let snapshot = fetch_updates_snapshot().await;
    let parameters = snapshot.parameters.expect("live parameter schema");
    assert!(!parameters.is_empty());
    let _caps = snapshot.caps.expect("live cap schema");
}

fn metadata() -> Value {
    json!([
        {"universe":[
            {"name":"BTC", "maxLeverage":40, "marginTableId":56},
            {"name":"OLD", "maxLeverage":5, "marginTableId":5, "isDelisted":true}
        ], "marginTables":[[56,{"marginTiers":[
            {"lowerBound":"0.0", "maxLeverage":40},
            {"lowerBound":"1000000.0", "maxLeverage":20}
        ]}]]},
        {"universe":[{"name":"xyz:BTC", "maxLeverage":10, "marginMode":"noCross"}], "marginTables":[]}
    ])
}

#[test]
fn updates_parameters_resolve_tiered_and_implicit_tables_and_retain_inactive_assets() {
    let parsed = parse_parameters(metadata()).expect("metadata");
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed["BTC"].margin_tiers.len(), 2);
    assert_eq!(parsed["OLD"].margin_tiers[0].max_leverage, 5);
    assert_eq!(
        parsed["xyz:BTC"].margin_mode,
        Some(MarketMarginMode::NoCross)
    );
    assert_eq!(parsed["BTC"].margin_mode, Some(MarketMarginMode::Normal));
    let mut legacy = metadata();
    legacy[0]["universe"][0]["onlyIsolated"] = json!(true);
    assert_eq!(
        parse_parameters(legacy).expect("legacy")["BTC"].margin_mode,
        None
    );
}

#[test]
fn updates_parameters_reject_missing_invalid_and_duplicate_data_as_a_family() {
    for bad in [
        json!([]),
        json!([{"universe":[],"marginTables":[]}]),
        json!({}),
    ] {
        assert!(parse_parameters(bad).is_err());
    }
    let mut missing = metadata();
    missing[0]["marginTables"] = json!([]);
    assert!(
        parse_parameters(missing).is_err(),
        "custom table must resolve"
    );
    let mut duplicate = metadata();
    duplicate[0]["universe"][1]["name"] = json!("BTC");
    assert!(parse_parameters(duplicate).is_err());
    let mut unordered = metadata();
    unordered[0]["marginTables"][0][1]["marginTiers"][1]["lowerBound"] = json!("0");
    assert!(parse_parameters(unordered).is_err());
    let mut invalid = metadata();
    invalid[0]["universe"][0]["maxLeverage"] = json!(0);
    assert!(parse_parameters(invalid).is_err());
    let mut unknown_mode = metadata();
    unknown_mode[1]["universe"][0]["marginMode"] = json!("futureMode");
    assert!(parse_parameters(unknown_mode).is_err());
}

#[test]
fn updates_parameters_reject_aggregate_tier_amplification() {
    let tiers: Vec<_> = (0..100)
        .map(|index| {
            json!({
                "lowerBound": index.to_string(),
                "maxLeverage": 100 - index,
            })
        })
        .collect();
    let universe: Vec<_> = (0..=MAX_UPDATE_MARGIN_TIERS / tiers.len())
        .map(|index| {
            json!({
                "name": format!("ASSET{index}"),
                "maxLeverage": 100,
                "marginTableId": 100,
            })
        })
        .collect();
    let raw = json!([{
        "universe": universe,
        "marginTables": [[100, {"marginTiers": tiers}]],
    }]);

    assert!(parse_parameters(raw).is_err());
}

#[test]
fn updates_caps_preserve_dex_identity_and_normalize_decimals() {
    let parsed = parse_caps(json!([null,
        {"name":"xyz","assetToStreamingOiCap":[["BTC","100.00"]]},
        {"name":"flx","assetToStreamingOiCap":[["flx:BTC","200"]]},
        {"name":"empty","assetToStreamingOiCap":[]}
    ]))
    .expect("caps");
    assert_eq!(parsed["xyz:BTC"].as_str(), "100");
    assert_eq!(parsed["flx:BTC"].as_str(), "200");
    assert!(!parsed.contains_key("BTC"));
    assert!(parse_caps(json!([null])).expect("no builders").is_empty());
}

#[test]
fn updates_caps_reject_partial_malformed_and_wrong_dex_values() {
    for bad in [
        json!([]),
        json!([null, null]),
        json!([null,{"name":"xyz"}]),
        json!([null,{"name":"xyz","assetToStreamingOiCap":[["flx:BTC","1"]]}]),
        json!([null,{"name":"xyz","assetToStreamingOiCap":[["BTC","-1"]]}]),
        json!([null,{"name":"xyz","assetToStreamingOiCap":[["BTC","NaN"]]}]),
        json!([null,{"name":"xyz","assetToStreamingOiCap":[["BTC","1"],["xyz:BTC","2"]]}]),
    ] {
        assert!(parse_caps(bad).is_err());
    }
}
