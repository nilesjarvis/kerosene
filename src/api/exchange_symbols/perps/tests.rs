use super::{append_perp_symbols, margin_mode_disallows_cross, parse_perp_dexes};
use crate::api::MarketType;

#[test]
fn registered_perp_dexes_include_empty_and_delisted_markets_without_listing_contracts() {
    let dexes = serde_json::json!([
        null, {"name": "active"}, {"name": "halted"}, {"name": "empty"}
    ]);
    let metas = serde_json::json!([
        {"universe": [{"name": "BTC"}]},
        {"collateralToken": 0, "universe": [{"name": "active:ABC"}]},
        {"collateralToken": 404, "universe": [{"name": "halted:ABC", "isDelisted": true}]},
        {"collateralToken": 7, "universe": []}
    ]);

    let registered = parse_perp_dexes(&dexes, &metas).expect("registered DEXes");
    assert_eq!(
        registered
            .iter()
            .map(|dex| (dex.name.as_str(), dex.collateral_token))
            .collect::<Vec<_>>(),
        vec![
            ("active", Some(0)),
            ("empty", Some(7)),
            ("halted", Some(404))
        ]
    );
    let mut symbols = Vec::new();
    append_perp_symbols(&mut symbols, &metas, &serde_json::json!([]), &dexes)
        .expect("tradable symbols");
    assert_eq!(
        symbols
            .iter()
            .map(|symbol| symbol.key.as_str())
            .collect::<Vec<_>>(),
        vec!["BTC", "active:ABC"]
    );
}

#[test]
fn registered_perp_dexes_normalize_names_without_guessing_missing_collateral() {
    let dexes = serde_json::json!([
        {"name": ""}, {"name": " NewDex "}, {"name": "newdex"}
    ]);
    let registered = parse_perp_dexes(&dexes, &serde_json::json!([{}]))
        .expect("registered DEX with pending market metadata");
    assert_eq!(registered.len(), 1);
    assert_eq!(registered[0].name, "newdex");
    assert_eq!(registered[0].collateral_token, None);
    assert!(parse_perp_dexes(&serde_json::json!([null, {}]), &serde_json::json!([])).is_err());
}

#[test]
fn margin_mode_strict_isolated_disallows_cross() {
    assert!(margin_mode_disallows_cross(&serde_json::json!({
        "marginMode": "strictIsolated"
    })));
}

#[test]
fn margin_mode_no_cross_disallows_cross() {
    assert!(margin_mode_disallows_cross(&serde_json::json!({
        "marginMode": "noCross"
    })));
}

#[test]
fn unknown_margin_mode_keeps_cross_allowed() {
    assert!(!margin_mode_disallows_cross(&serde_json::json!({
        "marginMode": "cross"
    })));
    assert!(!margin_mode_disallows_cross(&serde_json::json!({})));
}

#[test]
fn hip3_asset_index_uses_matching_dex_offset() {
    let mut symbols = Vec::new();

    append_perp_symbols(
        &mut symbols,
        &serde_json::json!([
            {
                "collateralToken": 0,
                "universe": [{ "name": "BTC", "szDecimals": 5, "maxLeverage": 50 }]
            },
            {
                "collateralToken": 1,
                "universe": [{ "name": "xyz:NVDA", "szDecimals": 2, "maxLeverage": 5 }]
            }
        ]),
        &serde_json::json!([]),
        &serde_json::json!([{ "name": "" }, { "name": "xyz" }]),
    )
    .expect("valid perp metadata");

    let hip3 = symbols
        .iter()
        .find(|symbol| symbol.key == "xyz:NVDA")
        .expect("hip3 symbol");
    assert_eq!(hip3.asset_index, 110_000);
    assert_eq!(hip3.collateral_token, Some(1));
    assert_eq!(hip3.market_type, MarketType::Perp);
}

#[test]
fn hip3_asset_index_fails_when_dex_metadata_is_missing() {
    let mut symbols = Vec::new();

    let err = append_perp_symbols(
        &mut symbols,
        &serde_json::json!([
            {
                "collateralToken": 0,
                "universe": [{ "name": "BTC", "szDecimals": 5, "maxLeverage": 50 }]
            },
            {
                "collateralToken": 1,
                "universe": [{ "name": "xyz:NVDA", "szDecimals": 2, "maxLeverage": 5 }]
            }
        ]),
        &serde_json::json!([]),
        &serde_json::json!([{ "name": "" }]),
    )
    .expect_err("missing dex metadata should fail");

    assert_eq!(
        err,
        "perpDexs metadata has 1 entries but allPerpMetas has 2; cannot build asset indices"
    );
    assert!(symbols.is_empty());
}

#[test]
fn hip3_growth_mode_defaults_false_when_flag_absent() {
    let mut symbols = Vec::new();

    append_perp_symbols(
        &mut symbols,
        &serde_json::json!([{
            "collateralToken": 0,
            "universe": [
                { "name": "xyz:NVDA", "szDecimals": 2, "maxLeverage": 5, "growthMode": "enabled" },
                { "name": "xyz:HCLI", "szDecimals": 2, "maxLeverage": 5 },
                { "name": "xyz:PLTR", "szDecimals": 2, "maxLeverage": 5, "growthMode": "disabled" }
            ]
        }]),
        &serde_json::json!([]),
        &serde_json::json!([{ "name": "" }]),
    )
    .expect("valid perp metadata");

    let growth_mode_by_key: Vec<(bool, &str)> = symbols
        .iter()
        .map(|symbol| (symbol.growth_mode, symbol.key.as_str()))
        .collect();
    assert_eq!(
        growth_mode_by_key,
        vec![(true, "xyz:NVDA"), (false, "xyz:HCLI"), (false, "xyz:PLTR")]
    );
}

#[test]
fn annotations_keep_last_duplicate_and_ignore_malformed_pairs() {
    let metas = serde_json::json!([{
        "universe": [
            {"name": "BTC"}, {"name": "builder:TEST"}, {"name": "ETH"}, {"name": "DOGE"}
        ]
    }]);
    let annotations = serde_json::json!([
        ["BTC", {"category": "old", "displayName": "old", "keywords": ["old"]}],
        ["BTC", {"category": "index", "displayName": "Bitcoin", "keywords": ["digital", 7, "asset"]}, "extra"],
        ["builder:TEST", {"category": "stocks", "displayName": "Test"}],
        ["builder:TEST", null],
        ["ETH", {"category": 42, "displayName": false, "keywords": 7}],
        null, [], ["BTC"], [7, {"category": "invalid"}]
    ]);
    let mut symbols = Vec::new();
    append_perp_symbols(
        &mut symbols,
        &metas,
        &annotations,
        &serde_json::json!([null]),
    )
    .expect("valid metadata");
    assert_eq!(symbols.len(), 4);
    assert_eq!(symbols[0].category, "index");
    assert_eq!(symbols[0].display_name.as_deref(), Some("Bitcoin"));
    assert_eq!(symbols[0].keywords, ["digital", "asset"]);
    for symbol in &symbols[1..] {
        assert_eq!(symbol.category, "crypto");
        assert!(symbol.display_name.is_none());
        assert!(symbol.keywords.is_empty());
    }
    assert_eq!(symbols[1].key, "builder:TEST");
    assert_eq!(symbols[1].ticker, "TEST");
}

#[test]
fn non_array_annotations_keep_default_symbol_fields() {
    let metas = serde_json::json!([{"universe": [{"name": "BTC"}]}]);
    let dexs = serde_json::json!([null]);
    let mut expected = Vec::new();
    append_perp_symbols(&mut expected, &metas, &serde_json::json!([]), &dexs)
        .expect("valid metadata");
    for annotations in [
        serde_json::json!(null),
        serde_json::json!({}),
        serde_json::json!("not an array"),
        serde_json::json!(false),
    ] {
        let mut actual = Vec::new();
        append_perp_symbols(&mut actual, &metas, &annotations, &dexs)
            .expect("optional annotations");
        assert_eq!(actual, expected);
    }
}
