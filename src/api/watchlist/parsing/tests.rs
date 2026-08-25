use super::*;
use std::collections::{HashMap, HashSet};

#[test]
fn watchlist_context_parser_accepts_finite_string_and_number_fields() {
    let raw = serde_json::json!([
        { "universe": [{ "name": "HYPE" }] },
        [{
            "funding": "0.001",
            "prevDayPx": 12.5,
            "dayNtlVlm": "1234.5",
            "openInterest": "42.5",
            "markPx": 20.0
        }]
    ]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(raw, None, &mut map);

    assert_eq!(parsed, Ok(1));
    let context = map.get("HYPE").expect("context");
    assert_eq!(context.funding, Some(0.001));
    assert_eq!(context.prev_day_px, Some(12.5));
    assert_eq!(context.mark_px, Some(20.0));
    assert_eq!(context.day_vlm, Some(1234.5));
    assert_eq!(context.open_interest_notional, Some(850.0));
}

#[test]
fn watchlist_context_parser_rejects_nonfinite_and_grouped_strings() {
    let raw = serde_json::json!([
        { "universe": [{ "name": "HYPE" }] },
        [{
            "funding": "NaN",
            "prevDayPx": "inf",
            "dayNtlVlm": "1,234",
            "openInterest": "-1",
            "markPx": "100"
        }]
    ]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(raw, None, &mut map);

    assert_eq!(parsed, Ok(1));
    let context = map.get("HYPE").expect("context");
    assert_eq!(context.funding, None);
    assert_eq!(context.prev_day_px, None);
    assert_eq!(context.mark_px, Some(100.0));
    assert_eq!(context.day_vlm, None);
    assert_eq!(context.open_interest_notional, None);
}

#[test]
fn watchlist_context_parser_rejects_invalid_or_overflowing_open_interest_notional() {
    let raw = serde_json::json!([
        {
            "universe": [
                { "name": "MISSING_MARK" },
                { "name": "ZERO_MARK" },
                { "name": "OVERFLOW" }
            ]
        },
        [
            { "openInterest": "10" },
            { "openInterest": "10", "markPx": "0" },
            { "openInterest": 1e308, "markPx": 1e308 }
        ]
    ]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(raw, None, &mut map);

    assert_eq!(parsed, Ok(3));
    for symbol in ["MISSING_MARK", "ZERO_MARK", "OVERFLOW"] {
        assert_eq!(
            map.get(symbol)
                .and_then(|context| context.open_interest_notional),
            None
        );
    }
}

#[test]
fn watchlist_context_parser_rejects_error_shaped_perp_payloads() {
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(
        serde_json::json!({ "error": "rate limited" }),
        None,
        &mut map,
    );

    assert!(parsed.is_err());
    assert!(map.is_empty());
}

#[test]
fn watchlist_context_parser_rejects_perp_payloads_missing_context_rows() {
    let raw = serde_json::json!([{ "universe": [{ "name": "HYPE" }] }, []]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(raw, None, &mut map);

    assert_eq!(
        parsed,
        Err("contexts array shorter than universe".to_string())
    );
    assert!(map.is_empty());
}

#[test]
fn watchlist_context_parser_rejects_error_shaped_spot_payloads() {
    let mut map = HashMap::new();

    let parsed = append_spot_contexts(serde_json::Value::Null, &mut map);

    assert!(parsed.is_err());
    assert!(map.is_empty());
}

#[test]
fn spot_context_parser_uses_context_coin_instead_of_universe_position() {
    let raw = serde_json::json!([
        { "universe": [{ "name": "@142", "index": 142 }] },
        [
            {
                "coin": "@140",
                "prevDayPx": "0.000037",
                "dayNtlVlm": "0.0"
            },
            {
                "coin": "@142",
                "prevDayPx": "58322.0",
                "markPx": "64154.2",
                "dayNtlVlm": "321.25"
            }
        ]
    ]);
    let mut map = HashMap::new();

    let parsed = append_spot_contexts(raw, &mut map);

    assert_eq!(parsed, Ok(1));
    let context = map.get("@142").expect("context");
    assert_eq!(context.prev_day_px, Some(58322.0));
    assert_eq!(context.mark_px, Some(64154.2));
    assert_eq!(context.day_vlm, Some(321.25));
    assert_eq!(context.open_interest_notional, None);
}

#[test]
fn spot_context_parser_keeps_positional_fallback_for_legacy_contexts_without_coin() {
    let raw = serde_json::json!([
        {
            "universe": [
                { "name": "PURR/USDC", "index": 0 },
                { "name": "@107", "index": 107 }
            ]
        },
        [
            {
                "prevDayPx": "0.9",
                "dayNtlVlm": "1234.0"
            },
            {
                "prevDayPx": "60.0",
                "dayNtlVlm": "987654.0"
            }
        ]
    ]);
    let mut map = HashMap::new();

    let parsed = append_spot_contexts(raw, &mut map);

    assert_eq!(parsed, Ok(2));
    let context = map.get("@107").expect("context");
    assert_eq!(context.prev_day_px, Some(60.0));
    assert_eq!(context.day_vlm, Some(987654.0));
}

#[test]
fn spot_context_parser_keys_api_named_pairs_by_their_name() {
    let raw = serde_json::json!([
        {
            "universe": [
                { "name": "PURR/USDC", "index": 0 },
                { "name": "@107", "index": 107 }
            ]
        },
        [
            {
                "coin": "PURR/USDC",
                "prevDayPx": "0.9",
                "dayNtlVlm": "1234.0"
            },
            {
                "coin": "@107",
                "prevDayPx": "60.0",
                "dayNtlVlm": "987654.0"
            }
        ]
    ]);
    let mut map = HashMap::new();

    let parsed = append_spot_contexts(raw, &mut map);

    assert_eq!(parsed, Ok(2));
    // The canonical pair is keyed by its API name in the symbol list, so
    // context lookups must resolve that key (the legacy "@0" stays too).
    let context = map.get("PURR/USDC").expect("named context");
    assert_eq!(context.prev_day_px, Some(0.9));
    assert_eq!(context.day_vlm, Some(1234.0));
    let legacy = map.get("@0").expect("legacy indexed context");
    assert_eq!(legacy.prev_day_px, Some(0.9));
}

#[test]
fn requested_spot_contexts_ignore_missing_unrelated_universe_rows() {
    let raw = serde_json::json!([
        {
            "universe": [
                { "name": "@142", "index": 142 },
                { "name": "@999", "index": 999 }
            ]
        },
        [{
            "coin": "@142",
            "prevDayPx": "58322.0",
            "dayNtlVlm": "321.25"
        }]
    ]);
    let requested = HashSet::from(["@142".to_string()]);
    let mut map = HashMap::new();

    let parsed = append_spot_contexts_for_symbols(raw, &requested, &mut map);

    assert_eq!(parsed, Ok(1));
    assert!(map.contains_key("@142"));
    assert!(!map.contains_key("@999"));
}

#[test]
fn requested_spot_contexts_return_available_rows_when_one_requested_row_is_missing() {
    let raw = serde_json::json!([
        {
            "universe": [
                { "name": "@142", "index": 142 },
                { "name": "@999", "index": 999 }
            ]
        },
        [{
            "coin": "@142",
            "prevDayPx": "58322.0",
            "dayNtlVlm": "321.25"
        }]
    ]);
    let requested = HashSet::from(["@142".to_string(), "@999".to_string()]);
    let mut map = HashMap::new();

    let parsed = append_spot_contexts_for_symbols(raw, &requested, &mut map);

    assert_eq!(parsed, Ok(1));
    assert!(map.contains_key("@142"));
    assert!(!map.contains_key("@999"));
}

#[test]
fn requested_perp_contexts_ignore_missing_unrelated_universe_rows() {
    let raw = serde_json::json!([
        {
            "universe": [
                { "name": "HYPE" },
                { "name": "UNRELATED" }
            ]
        },
        [{
            "funding": "0.001",
            "prevDayPx": "40.0",
            "dayNtlVlm": "123.0"
        }]
    ]);
    let requested = HashSet::from(["HYPE".to_string()]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts_for_symbols(raw, None, &requested, &mut map);

    assert_eq!(parsed, Ok(1));
    assert!(map.contains_key("HYPE"));
    assert!(!map.contains_key("UNRELATED"));
}
