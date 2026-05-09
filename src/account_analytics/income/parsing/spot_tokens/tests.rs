use super::parse_spot_token_names;

#[test]
fn spot_token_names_ignore_missing_or_empty_names() {
    let raw = serde_json::json!({
        "tokens": [
            { "index": 0, "name": "USDC" },
            { "index": 1, "name": "" },
            { "index": "bad", "name": "BAD" },
            { "index": 2, "name": "BTC" }
        ]
    });

    let names = parse_spot_token_names(&raw);
    assert_eq!(names.get(&0).map(String::as_str), Some("USDC"));
    assert_eq!(names.get(&2).map(String::as_str), Some("BTC"));
    assert!(!names.contains_key(&1));
}
