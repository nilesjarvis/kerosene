use super::*;

#[test]
fn appends_question_bucket_and_fallback_metadata() {
    let mut symbols = Vec::new();
    append_outcome_symbols(
        &mut symbols,
        outcome_meta_from_json(serde_json::json!({
            "outcomes": [
                {
                    "outcome": 66,
                    "name": "Recurring Fallback",
                    "description": "other",
                    "sideSpecs": [{"name": "Yes"}, {"name": "No"}]
                },
                {
                    "outcome": 67,
                    "name": "Recurring Named Outcome",
                    "description": "index:0",
                    "sideSpecs": [{"name": "Yes"}, {"name": "No"}]
                }
            ],
            "questions": [{
                "question": 12,
                "name": "Recurring",
                "description": concat!(
                    "class:priceBucket|underlying:BTC|expiry:20260520-0600|",
                    "priceThresholds:75348,78423|period:1d"
                ),
                "fallbackOutcome": 66,
                "namedOutcomes": [67, 68, 69],
                "settledNamedOutcomes": []
            }]
        })),
    );

    let fallback = outcome_by_key_or_panic(&symbols, "#660");
    let bucket = outcome_by_key_or_panic(&symbols, "#670");

    assert_eq!(fallback.question_id, Some(12));
    assert!(fallback.is_question_fallback);
    assert!(!symbol_by_key_or_panic(&symbols, "#660").is_user_selectable_market());
    assert_eq!(bucket.bucket_index, Some(0));
    assert!(!bucket.is_question_fallback);
    assert_eq!(
        bucket.question_price_thresholds,
        vec!["75348".to_string(), "78423".to_string()]
    );
    assert_eq!(bucket.question_named_outcomes, vec![67, 68, 69]);
    assert_eq!(bucket.question_fallback_outcome, Some(66));

    let bucket_symbol = symbol_by_key_or_panic(&symbols, "#670");
    assert!(bucket_symbol.is_user_selectable_market());
    assert_eq!(
        bucket_symbol.display_name.as_deref(),
        Some("YES: BTC is below 75,348 at 2026-05-20 06:00 UTC")
    );
}
