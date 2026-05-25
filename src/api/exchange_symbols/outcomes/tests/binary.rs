use super::*;

#[test]
fn appends_binary_outcome_symbol_metadata() {
    let mut symbols = Vec::new();
    append_outcome_symbols(
        &mut symbols,
        outcome_meta_from_json(serde_json::json!({
            "outcomes": [{
                "outcome": 65,
                "name": "Recurring",
                "description": concat!(
                    "class:priceBinary|underlying:BTC|expiry:20260520-0600|",
                    "targetPrice:76886|period:1d"
                ),
                "sideSpecs": [{"name": "Yes"}, {"name": "No"}]
            }],
            "questions": []
        })),
    );

    let yes = symbol_by_key_or_panic(&symbols, "#650");
    let info = outcome_by_key_or_panic(&symbols, "#650");

    assert_eq!(yes.asset_index, 100_000_650);
    assert_eq!(info.encoding, 650);
    assert_eq!(info.side_index, 0);
    assert_eq!(info.class.as_deref(), Some("priceBinary"));
    assert_eq!(info.underlying.as_deref(), Some("BTC"));
    assert_eq!(info.target_price.as_deref(), Some("76886"));
    assert!(info.question_id.is_none());
    assert_eq!(
        yes.display_name.as_deref(),
        Some("YES: BTC is above 76,886 at 2026-05-20 06:00 UTC")
    );
}

#[test]
fn skips_non_binary_outcome_sides() {
    let mut symbols = Vec::new();
    append_outcome_symbols(
        &mut symbols,
        outcome_meta_from_json(serde_json::json!({
            "outcomes": [{
                "outcome": 65,
                "name": "Recurring",
                "description": concat!(
                    "class:priceBinary|underlying:BTC|expiry:20260520-0600|",
                    "targetPrice:76886|period:1d"
                ),
                "sideSpecs": [{"name": "Yes"}, {"name": "No"}, {"name": "Maybe"}]
            }],
            "questions": []
        })),
    );

    assert!(symbols.iter().any(|symbol| symbol.key == "#650"));
    assert!(symbols.iter().any(|symbol| symbol.key == "#651"));
    assert!(!symbols.iter().any(|symbol| symbol.key == "#652"));
}
