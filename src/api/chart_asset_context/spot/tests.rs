use super::*;
use serde_json::json;

fn parse_spot_chart_asset_context(response: &Value, symbol: &str) -> Option<AssetContext> {
    SpotContextLookup::new(response).ok()?.get(symbol)
}

fn batch_mid_prices(response: &Value, symbols: &[&str]) -> Vec<(String, Option<String>)> {
    parse_spot_chart_asset_contexts(
        response,
        symbols.iter().map(|symbol| symbol.to_string()).collect(),
    )
    .expect("valid response")
    .into_iter()
    .map(|(symbol, context)| (symbol, context.mid_px))
    .collect()
}

#[test]
fn batch_preserves_first_universe_match_alias_collisions_and_request_order() {
    let response = json!([
        { "universe": [
            { "index": 0, "name": "ALIAS/USDC" },
            { "index": 0, "name": "OTHER/USDC" },
            { "index": 2, "name": "ALIAS/USDC" },
            { "index": 3, "name": "@2" },
            { "index": "4", "name": "SKIP/USDC" },
            { "index": 5 }
        ] },
        [
            { "midPx": "10" }, { "midPx": "20" }, { "midPx": "30" },
            { "midPx": "40" }, { "midPx": "50" }, { "midPx": "60" }
        ]
    ]);
    let actual = batch_mid_prices(
        &response,
        &[
            "@3",
            "@0",
            "ALIAS/USDC",
            "OTHER/USDC",
            "@2",
            "@0",
            "SKIP/USDC",
            "@5",
            "@00",
            "missing",
        ],
    );
    let expected = [
        ("@3", "40"),
        ("@0", "10"),
        ("ALIAS/USDC", "10"),
        ("OTHER/USDC", "20"),
        ("@2", "30"),
        ("@5", "60"),
    ]
    .map(|(symbol, mid)| (symbol.to_string(), Some(mid.to_string())));
    assert_eq!(actual, expected);
}

#[test]
fn keyed_contexts_preserve_duplicate_and_alias_priority_without_unkeyed_fallback() {
    let mut response = json!([
        { "universe": [{ "index": 7, "name": "PAIR/USDC" }, { "index": 8 }] },
        [
            { "coin": "PAIR/USDC", "midPx": "10" },
            { "midPx": "unkeyed" },
            { "coin": "@7", "midPx": "20" },
            { "coin": "@7", "midPx": "30" }
        ]
    ]);
    assert_eq!(
        batch_mid_prices(&response, &["@7", "PAIR/USDC", "@8"]),
        vec![
            ("@7".to_string(), Some("30".to_string())),
            ("PAIR/USDC".to_string(), Some("10".to_string())),
        ]
    );
    response[1]
        .as_array_mut()
        .expect("context array")
        .push(json!({ "coin": "@7", "midPx": 99 }));
    assert_eq!(
        batch_mid_prices(&response, &["@7", "PAIR/USDC"]),
        vec![("PAIR/USDC".to_string(), Some("10".to_string())),],
        "a malformed selected context must not fall back to the valid alias"
    );
}

#[test]
fn batch_validation_keeps_error_messages_and_skips_only_invalid_selected_contexts() {
    for (response, reason) in [
        (json!({}), "expected [meta, contexts]"),
        (json!([{}, []]), "missing meta universe"),
        (
            json!([{ "universe": [{}] }, {}]),
            "contexts must be an array",
        ),
        (
            json!([{ "universe": [] }, [{}]]),
            "empty universe or contexts",
        ),
        (
            json!([{ "universe": [{}] }, []]),
            "empty universe or contexts",
        ),
    ] {
        let error = parse_spot_chart_asset_contexts(&response, vec!["@0".to_string()])
            .expect_err("invalid schema");
        assert_eq!(
            error,
            format!("spotMetaAndAssetCtxs schema invalid: {reason}")
        );
    }
    let response = json!([
        { "universe": [{ "index": 0 }, { "index": 1 }, { "index": 2 }, { "index": 3 }] },
        [null, { "midPx": 99 }, { "midPx": "20" }]
    ]);
    assert_eq!(
        batch_mid_prices(&response, &["@0", "@1", "@2", "@3"]),
        vec![("@2".to_string(), Some("20".to_string())),]
    );
}

#[test]
fn rejects_error_shaped_spot_context_response() {
    for invalid in [
        json!({ "error": "rate limited" }),
        json!([{}, []]),
        json!([{ "universe": [{ "name": "@107", "index": 107 }] }, {}]),
    ] {
        assert!(SpotContextLookup::new(&invalid).is_err());
    }
}

#[test]
fn parses_spot_context_by_at_index() {
    let resp = json!([
        {
            "universe": [
                { "name": "PURR/USDC", "index": 0 },
                { "name": "HYPE/USDC", "index": 107 }
            ]
        },
        [
            { "midPx": "1.0", "prevDayPx": "0.9", "dayNtlVlm": "1234.0",
              "dayBaseVlm": "567.0" },
            { "midPx": "62.1", "prevDayPx": "60.0", "dayNtlVlm": "987654.0",
              "dayBaseVlm": "15555.0" }
        ]
    ]);

    let ctx = parse_spot_chart_asset_context(&resp, "@107").expect("spot context");

    assert_eq!(ctx.mid_px.as_deref(), Some("62.1"));
    assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("987654.0"));
    assert_eq!(ctx.day_base_vlm.as_deref(), Some("15555.0"));
    assert!(ctx.funding.is_none());
    assert!(ctx.open_interest.is_none());
}

#[test]
fn parses_spot_context_by_api_pair_name() {
    // The canonical pair is keyed by its API name ("PURR/USDC"), not by
    // its "@{index}" form, and must still resolve its spot context.
    let resp = json!([
        {
            "universe": [
                { "name": "PURR/USDC", "index": 0 },
                { "name": "HYPE/USDC", "index": 107 }
            ]
        },
        [
            { "midPx": "1.0", "prevDayPx": "0.9", "dayNtlVlm": "1234.0",
              "dayBaseVlm": "567.0" },
            { "midPx": "62.1", "prevDayPx": "60.0", "dayNtlVlm": "987654.0",
              "dayBaseVlm": "15555.0" }
        ]
    ]);

    let ctx = parse_spot_chart_asset_context(&resp, "PURR/USDC").expect("spot context");

    assert_eq!(ctx.mid_px.as_deref(), Some("1.0"));
    assert_eq!(ctx.prev_day_px.as_deref(), Some("0.9"));
    assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("1234.0"));
}

#[test]
fn parses_spot_context_by_context_coin_when_universe_position_differs_from_index() {
    let resp = json!([
        {
            "universe": [
                { "name": "@142", "index": 142 }
            ]
        },
        [
            { "coin": "@140", "midPx": "0.000068", "prevDayPx": "0.000037" },
            { "coin": "@142", "midPx": "60105.5", "prevDayPx": "58322.0",
              "dayNtlVlm": "32176298.0", "dayBaseVlm": "546.48611" }
        ]
    ]);

    let ctx = parse_spot_chart_asset_context(&resp, "@142").expect("spot context");

    assert_eq!(ctx.mid_px.as_deref(), Some("60105.5"));
    assert_eq!(ctx.prev_day_px.as_deref(), Some("58322.0"));
    assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("32176298.0"));
    assert_eq!(ctx.day_base_vlm.as_deref(), Some("546.48611"));
}

#[test]
fn spot_context_returns_none_when_symbol_absent_or_malformed() {
    let resp = json!([
        { "universe": [ { "name": "PURR/USDC", "index": 0 } ] },
        [ { "midPx": "1.0" } ]
    ]);

    assert!(parse_spot_chart_asset_context(&resp, "@107").is_none());
    assert!(parse_spot_chart_asset_context(&json!({}), "@107").is_none());
    assert!(parse_spot_chart_asset_context(&json!([{}]), "@107").is_none());
}
