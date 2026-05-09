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
    let mut question_names_by_outcome: HashMap<u32, (u32, String)> = HashMap::new();
    for question in &outcome_meta.questions {
        for outcome_id in question
            .named_outcomes
            .iter()
            .chain(question.settled_named_outcomes.iter())
            .copied()
            .chain(question.fallback_outcome)
        {
            question_names_by_outcome
                .entry(outcome_id)
                .or_insert_with(|| (question.question, question.name.clone()));
        }
    }

    for outcome in outcome_meta.outcomes {
        let description_parts = parse_outcome_description(&outcome.description);
        let question = question_names_by_outcome.get(&outcome.outcome);
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
            keywords.sort();
            keywords.dedup();

            let info = OutcomeSymbolInfo {
                outcome_id: outcome.outcome,
                question_id: question.map(|(id, _)| *id),
                question_name: question.map(|(_, name)| name.clone()),
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
            let market_display_name = question
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| outcome.name.clone());

            symbols.push(ExchangeSymbol {
                key,
                ticker: format!("OUT{}-{}", outcome.outcome, side_name.to_uppercase()),
                category: "outcome".to_string(),
                display_name: Some(format!("{} - {}", market_display_name, side_name)),
                keywords,
                asset_index: outcome_asset_index(encoding),
                sz_decimals: 0,
                max_leverage: 1,
                only_isolated: true,
                market_type: MarketType::Outcome,
                outcome: Some(info),
            });
        }
    }
}
