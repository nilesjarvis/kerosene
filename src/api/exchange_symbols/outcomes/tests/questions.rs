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

#[test]
fn question_groups_preserve_each_members_complete_metadata_and_input_order() {
    let entries = [
        (42, 2),
        (45, 1),
        (40, 0),
        (46, 0),
        (44, 0),
        (41, 1),
        (43, 0),
    ];
    let meta = outcome_meta_from_json(serde_json::json!({
        "feeScale": "1.25",
        "outcomes": entries.map(|(outcome, index)| serde_json::json!({
            "outcome": outcome,
            "name": "Recurring Named Outcome",
            "description": if outcome == 46 {
                "class:priceBinary|underlying:SOL|targetPrice:100|expiry:20300101-0000".to_string()
            } else {
                format!("index:{index}")
            },
            "sideSpecs": [{"name": "Yes"}, {"name": "No"}],
            "quoteToken": "USDC",
            "deployerFeeScale": "0.5"
        })),
        "questions": [
            {"question": 10, "name": "Unused", "description": "ignored"},
            {
                "question": 11,
                "name": "BTC buckets",
                "description": "class:priceBucket|underlying:BTC|expiry:20300101-0000|priceThresholds:100, 200|period:1d",
                "namedOutcomes": [42, 40, 42],
                "settledNamedOutcomes": [41, 40],
                "fallbackOutcome": 43
            },
            {
                "question": 12,
                "name": "ETH buckets",
                "description": "class:priceBucket|underlying:ETH|expiry:20310101-0000|priceThresholds:10, 20|period:1h",
                "namedOutcomes": [44],
                "settledNamedOutcomes": [45]
            }
        ]
    }));
    let symbols = parse_outcome_symbols(meta.clone(), &[]).expect("valid question groups");
    assert_eq!(symbols.len(), entries.len() * 2);

    for (index, entry) in meta.outcomes.iter().enumerate() {
        // Isolating a market and its parent must produce the same complete symbols,
        // regardless of other groups or repeated memberships in the full response.
        let question = meta.questions.iter().find(|question| {
            question.named_outcomes.contains(&entry.outcome)
                || question.settled_named_outcomes.contains(&entry.outcome)
                || question.fallback_outcome == Some(entry.outcome)
        });
        let isolated = parse_outcome_symbols(
            OutcomeMetaResponse {
                outcomes: vec![entry.clone()],
                questions: question.cloned().into_iter().collect(),
                fee_scale: meta.fee_scale.clone(),
            },
            &[],
        )
        .expect("isolated market");
        assert_eq!(&symbols[index * 2..index * 2 + 2], isolated);
        for (side, symbol) in symbols[index * 2..index * 2 + 2].iter().enumerate() {
            assert_eq!(symbol.key, format!("#{}", entry.outcome * 10 + side as u32));
            let info = symbol.outcome.as_ref().expect("outcome metadata");
            assert_eq!(info.question_id, question.map(|question| question.question));
            assert_eq!(
                info.question_name.as_deref(),
                question.map(|question| question.name.as_str())
            );
            assert_eq!(
                info.question_description.as_deref(),
                question.map(|question| question.description.as_str())
            );
            assert_eq!(
                info.question_named_outcomes,
                question
                    .map(|question| question.named_outcomes.clone())
                    .unwrap_or_default()
            );
            assert_eq!(
                info.question_settled_named_outcomes,
                question
                    .map(|question| question.settled_named_outcomes.clone())
                    .unwrap_or_default()
            );
            assert_eq!(
                info.question_fallback_outcome,
                question.and_then(|question| question.fallback_outcome)
            );
            let blocked_reason = match entry.outcome {
                40 | 41 | 45 => Some("Settled"),
                43 => Some("Fallback settlement contract"),
                _ => None,
            };
            assert_eq!(info.trading_block_reason(0), blocked_reason);
            assert_eq!(symbol.is_user_selectable_market(), entry.outcome != 43);
        }
    }
}
