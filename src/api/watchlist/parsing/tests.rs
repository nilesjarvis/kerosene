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

    append_perp_contexts(raw, None, &mut map);

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

    append_perp_contexts(raw, None, &mut map);

    let context = map.get("HYPE").expect("context");
    assert_eq!(context.funding, None);
    assert_eq!(context.prev_day_px, None);
    assert_eq!(context.day_vlm, None);
}
