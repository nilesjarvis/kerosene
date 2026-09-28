use super::parse_chart_asset_context;
use serde_json::json;

fn hip3_response() -> serde_json::Value {
    json!([
        {
            "universe": [
                { "name": "xyz:TSLA" },
                { "name": "xyz:NVDA" }
            ]
        },
        [
            { "funding": "0.0000125", "openInterest": "100.0", "dayNtlVlm": "111.0",
              "dayBaseVlm": "2.0", "markPx": "400.0", "midPx": "400.5",
              "oraclePx": "399.5", "prevDayPx": "395.0", "impactPxs": ["400.0", "401.0"] },
            { "funding": "0.0000125", "openInterest": "11560.744", "dayNtlVlm": "987654.0",
              "dayBaseVlm": "33.0", "markPx": "120.0", "midPx": "120.1",
              "oraclePx": "119.9", "prevDayPx": "118.0", "impactPxs": ["120.0", "120.2"] }
        ]
    ])
}

#[test]
fn parses_hip3_context_by_full_dex_coin_name() {
    let resp = hip3_response();
    let ctx =
        parse_chart_asset_context(&resp, "xyz:NVDA", Some("xyz")).expect("context for xyz:NVDA");
    assert_eq!(ctx.open_interest.as_deref(), Some("11560.744"));
    assert_eq!(ctx.day_ntl_vlm.as_deref(), Some("987654.0"));
    assert_eq!(ctx.day_base_vlm.as_deref(), Some("33.0"));
    assert!(ctx.funding.is_some());
}

#[test]
fn matches_bare_universe_names_via_dex_reprefix() {
    // Some per-dex responses name coins bare ("NVDA"); they must still match
    // the canonical "xyz:NVDA" chart symbol after re-prefixing.
    let resp = json!([
        { "universe": [ { "name": "TSLA" }, { "name": "NVDA" } ] },
        [
            { "openInterest": "1.0", "dayNtlVlm": "2.0" },
            { "openInterest": "11560.744", "dayNtlVlm": "987654.0" }
        ]
    ]);
    let ctx = parse_chart_asset_context(&resp, "xyz:NVDA", Some("xyz"))
        .expect("re-prefixed match for xyz:NVDA");
    assert_eq!(ctx.open_interest.as_deref(), Some("11560.744"));
}

#[test]
fn parses_main_dex_context_without_dex_prefix() {
    let resp = json!([
        { "universe": [ { "name": "BTC" }, { "name": "ETH" } ] },
        [
            { "openInterest": "5000.0", "dayNtlVlm": "9.0" },
            { "openInterest": "6000.0", "dayNtlVlm": "8.0" }
        ]
    ]);
    let ctx = parse_chart_asset_context(&resp, "ETH", None).expect("context for ETH");
    assert_eq!(ctx.open_interest.as_deref(), Some("6000.0"));
}

#[test]
fn returns_none_when_symbol_absent_from_universe() {
    let resp = hip3_response();
    assert!(parse_chart_asset_context(&resp, "xyz:AAPL", Some("xyz")).is_none());
}

#[test]
fn returns_none_for_malformed_response() {
    assert!(parse_chart_asset_context(&json!({}), "BTC", None).is_none());
    assert!(parse_chart_asset_context(&json!([{}]), "BTC", None).is_none());
}

#[test]
fn context_decoding_keeps_optional_strings_and_rejects_malformed_fields() {
    let mut response = json!([
        { "universe": [{ "name": "BTC" }, { "name": "BTC" }] },
        [
            { "midPx": null, "impactPxs": ["100", "101"], "ignored": { "nested": [true] } },
            { "midPx": "102" }
        ]
    ]);
    let context = parse_chart_asset_context(&response, "BTC", None).expect("optional fields");
    assert!(context.mid_px.is_none());
    assert!(context.funding.is_none());
    assert_eq!(context.impact_pxs, Some(vec!["100".into(), "101".into()]));

    for malformed in [
        json!(null),
        json!({ "midPx": 100 }),
        json!({ "funding": true }),
        json!({ "impactPxs": ["100", 101] }),
    ] {
        response[1][0] = malformed;
        assert!(
            parse_chart_asset_context(&response, "BTC", None).is_none(),
            "a malformed first match must not fall back to another universe entry"
        );
    }
}

#[test]
fn context_lookup_preserves_literal_dex_prefix_and_qualified_name_matching() {
    for (name, dex, symbol, matches) in [
        ("BTC", None, "BTC", true),
        ("BTC", None, "btc", false),
        ("BTC", Some("xyz"), "xyz:BTC", true),
        ("BTC", Some("xyz"), "BTC", false),
        ("BTC", Some("xyz"), "xyzz:BTC", false),
        ("xyz:BTC", None, "xyz:BTC", true),
        ("xyz:BTC", Some("other"), "xyz:BTC", true),
        ("xyz:BTC", Some("xyz"), "BTC", false),
        ("xyz:BTC", Some("xyz"), "xyz:xyz:BTC", false),
        ("BTC", Some("x:y"), "x:y:BTC", true),
        ("BTC", Some(""), ":BTC", true),
        ("BTC", Some(""), "BTC", false),
        ("", Some(""), ":", true),
        ("", None, "", true),
        (":BTC", Some("xyz"), ":BTC", true),
        (" btc ", Some(" xyz "), " xyz : btc ", true),
        ("Ω", Some("界"), "界:Ω", true),
    ] {
        let response = json!([
            { "universe": [null, { "name": 7 }, { "name": name }] },
            [{ "midPx": "99" }, { "midPx": "100" }, { "midPx": "101" }]
        ]);
        let context = parse_chart_asset_context(&response, symbol, dex);
        assert_eq!(
            context.and_then(|context| context.mid_px).as_deref(),
            matches.then_some("101"),
            "name={name:?}, dex={dex:?}, symbol={symbol:?}"
        );
    }
}
