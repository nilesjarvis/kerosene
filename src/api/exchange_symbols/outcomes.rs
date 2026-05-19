mod description;
mod encoding;

use super::model::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::api::USDH_TOKEN_INDEX;

use description::parse_outcome_description;
use encoding::{outcome_asset_index, outcome_coin_key, outcome_encoding};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct OutcomeMetaResponse {
    #[serde(default)]
    outcomes: Vec<OutcomeMetaEntry>,
    #[serde(default)]
    questions: Vec<OutcomeQuestionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct OutcomeMetaEntry {
    outcome: u32,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "sideSpecs")]
    side_specs: Vec<OutcomeSideSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct OutcomeSideSpec {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OutcomeQuestionEntry {
    question: u32,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "namedOutcomes")]
    named_outcomes: Vec<u32>,
    #[serde(default, rename = "settledNamedOutcomes")]
    settled_named_outcomes: Vec<u32>,
    #[serde(default, rename = "fallbackOutcome")]
    fallback_outcome: Option<u32>,
}

pub(super) fn append_outcome_symbols(
    symbols: &mut Vec<ExchangeSymbol>,
    outcome_meta: OutcomeMetaResponse,
) {
    let mut questions_by_outcome: HashMap<u32, OutcomeQuestionInfo> = HashMap::new();
    for question in &outcome_meta.questions {
        let info = OutcomeQuestionInfo::from_entry(question);
        for outcome_id in question
            .named_outcomes
            .iter()
            .chain(question.settled_named_outcomes.iter())
            .copied()
            .chain(question.fallback_outcome)
        {
            questions_by_outcome
                .entry(outcome_id)
                .or_insert_with(|| info.clone());
        }
    }

    for outcome in outcome_meta.outcomes {
        let description_parts = parse_outcome_description(&outcome.description);
        let question = questions_by_outcome.get(&outcome.outcome);
        for (side_index, side) in outcome.side_specs.iter().enumerate() {
            if side_index > 9 {
                continue;
            }

            let side_index = side_index as u32;
            let side_name = if side.name.trim().is_empty() {
                format!("Side {side_index}")
            } else {
                side.name.trim().to_string()
            };
            let encoding = outcome_encoding(outcome.outcome, side_index);
            let key = outcome_coin_key(encoding);
            let mut keywords = vec![
                "outcome".to_string(),
                "prediction".to_string(),
                "usdh".to_string(),
                side_name.to_lowercase(),
                outcome.name.to_lowercase(),
                outcome.description.to_lowercase(),
            ];
            for value in description_parts.values() {
                keywords.push(value.to_lowercase());
            }
            let info = OutcomeSymbolInfo {
                outcome_id: outcome.outcome,
                question_id: question.map(|question| question.question_id),
                question_name: question.map(|question| question.name.clone()),
                question_description: question.map(|question| question.description.clone()),
                question_class: question.and_then(|question| question.class.clone()),
                question_underlying: question.and_then(|question| question.underlying.clone()),
                question_expiry: question.and_then(|question| question.expiry.clone()),
                question_price_thresholds: question
                    .map(|question| question.price_thresholds.clone())
                    .unwrap_or_default(),
                question_period: question.and_then(|question| question.period.clone()),
                question_named_outcomes: question
                    .map(|question| question.named_outcomes.clone())
                    .unwrap_or_default(),
                question_settled_named_outcomes: question
                    .map(|question| question.settled_named_outcomes.clone())
                    .unwrap_or_default(),
                question_fallback_outcome: question.and_then(|question| question.fallback_outcome),
                bucket_index: description_parts
                    .get("index")
                    .and_then(|value| value.parse::<u32>().ok()),
                is_question_fallback: question
                    .and_then(|question| question.fallback_outcome)
                    .is_some_and(|fallback| fallback == outcome.outcome),
                side_index,
                side_name: side_name.clone(),
                outcome_name: outcome.name.clone(),
                description: outcome.description.clone(),
                class: description_parts.get("class").cloned(),
                underlying: description_parts.get("underlying").cloned(),
                expiry: description_parts.get("expiry").cloned(),
                target_price: description_parts.get("targetPrice").cloned(),
                period: description_parts.get("period").cloned(),
                quote_symbol: "USDH".to_string(),
                quote_token_index: Some(USDH_TOKEN_INDEX),
                encoding,
            };
            let display_name = info.display_label();
            keywords.push(info.market_label().to_lowercase());
            keywords.push(display_name.to_lowercase());
            keywords.push(info.side_condition_label().to_lowercase());
            keywords.sort();
            keywords.dedup();

            symbols.push(ExchangeSymbol {
                key,
                ticker: format!("OUT{}-{}", outcome.outcome, side_name.to_uppercase()),
                category: "outcome".to_string(),
                display_name: Some(display_name),
                keywords,
                asset_index: outcome_asset_index(encoding),
                collateral_token: None,
                sz_decimals: 0,
                max_leverage: 1,
                only_isolated: true,
                market_type: MarketType::Outcome,
                outcome: Some(info),
            });
        }
    }
}

#[derive(Debug, Clone)]
struct OutcomeQuestionInfo {
    question_id: u32,
    name: String,
    description: String,
    class: Option<String>,
    underlying: Option<String>,
    expiry: Option<String>,
    price_thresholds: Vec<String>,
    period: Option<String>,
    named_outcomes: Vec<u32>,
    settled_named_outcomes: Vec<u32>,
    fallback_outcome: Option<u32>,
}

impl OutcomeQuestionInfo {
    fn from_entry(entry: &OutcomeQuestionEntry) -> Self {
        let description_parts = parse_outcome_description(&entry.description);
        let price_thresholds = description_parts
            .get("priceThresholds")
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        Self {
            question_id: entry.question,
            name: entry.name.clone(),
            description: entry.description.clone(),
            class: description_parts.get("class").cloned(),
            underlying: description_parts.get("underlying").cloned(),
            expiry: description_parts.get("expiry").cloned(),
            price_thresholds,
            period: description_parts.get("period").cloned(),
            named_outcomes: entry.named_outcomes.clone(),
            settled_named_outcomes: entry.settled_named_outcomes.clone(),
            fallback_outcome: entry.fallback_outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome_meta_from_json(value: serde_json::Value) -> OutcomeMetaResponse {
        serde_json::from_value(value).expect("valid outcome meta fixture")
    }

    #[test]
    fn appends_binary_outcome_symbol_metadata() {
        let mut symbols = Vec::new();
        append_outcome_symbols(
            &mut symbols,
            outcome_meta_from_json(serde_json::json!({
                "outcomes": [{
                    "outcome": 65,
                    "name": "Recurring",
                    "description": "class:priceBinary|underlying:BTC|expiry:20260520-0600|targetPrice:76886|period:1d",
                    "sideSpecs": [{"name": "Yes"}, {"name": "No"}]
                }],
                "questions": []
            })),
        );

        let yes = symbols
            .iter()
            .find(|symbol| symbol.key == "#650")
            .expect("yes side");
        let info = yes.outcome.as_ref().expect("outcome metadata");

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
                    "description": "class:priceBucket|underlying:BTC|expiry:20260520-0600|priceThresholds:75348,78423|period:1d",
                    "fallbackOutcome": 66,
                    "namedOutcomes": [67, 68, 69],
                    "settledNamedOutcomes": []
                }]
            })),
        );

        let fallback = symbols
            .iter()
            .find(|symbol| symbol.key == "#660")
            .and_then(|symbol| symbol.outcome.as_ref())
            .expect("fallback side");
        let bucket = symbols
            .iter()
            .find(|symbol| symbol.key == "#670")
            .and_then(|symbol| symbol.outcome.as_ref())
            .expect("bucket side");

        assert_eq!(fallback.question_id, Some(12));
        assert!(fallback.is_question_fallback);
        assert!(
            !symbols
                .iter()
                .find(|symbol| symbol.key == "#660")
                .expect("fallback symbol")
                .is_user_selectable_market()
        );
        assert_eq!(bucket.bucket_index, Some(0));
        assert!(!bucket.is_question_fallback);
        assert_eq!(
            bucket.question_price_thresholds,
            vec!["75348".to_string(), "78423".to_string()]
        );
        assert_eq!(bucket.question_named_outcomes, vec![67, 68, 69]);
        assert_eq!(bucket.question_fallback_outcome, Some(66));

        let bucket_symbol = symbols
            .iter()
            .find(|symbol| symbol.key == "#670")
            .expect("bucket symbol");
        assert!(bucket_symbol.is_user_selectable_market());
        assert_eq!(
            bucket_symbol.display_name.as_deref(),
            Some("YES: BTC is below 75,348 at 2026-05-20 06:00 UTC")
        );
    }
}
