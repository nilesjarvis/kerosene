use super::*;
use std::collections::HashMap;

#[test]
fn watchlist_context_parser_accepts_finite_string_and_number_fields() {
    let raw = serde_json::json!([
        { "universe": [{ "name": "HYPE" }] },
        [{
            "funding": "0.001",
            "prevDayPx": 12.5,
            "dayNtlVlm": "1234.5"
        }]
    ]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(raw, None, &mut map);

    assert_eq!(parsed, Ok(1));
    let context = map.get("HYPE").expect("context");
    assert_eq!(context.funding, Some(0.001));
    assert_eq!(context.prev_day_px, Some(12.5));
    assert_eq!(context.day_vlm, Some(1234.5));
}

#[test]
fn watchlist_context_parser_rejects_nonfinite_and_grouped_strings() {
    let raw = serde_json::json!([
        { "universe": [{ "name": "HYPE" }] },
        [{
            "funding": "NaN",
            "prevDayPx": "inf",
            "dayNtlVlm": "1,234"
        }]
    ]);
    let mut map = HashMap::new();

    let parsed = append_perp_contexts(raw, None, &mut map);

    assert_eq!(parsed, Ok(1));
    let context = map.get("HYPE").expect("context");
    assert_eq!(context.funding, None);
    assert_eq!(context.prev_day_px, None);
    assert_eq!(context.day_vlm, None);
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
