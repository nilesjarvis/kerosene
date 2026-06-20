mod description;
mod encoding;
mod questions;

use super::model::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::api::{USDC_TOKEN_INDEX, USDH_TOKEN_INDEX};

use description::parse_outcome_description;
use encoding::{outcome_asset_index, outcome_coin_key, outcome_encoding};
use questions::questions_by_outcome;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub(super) struct OutcomeMetaResponse {
    #[serde(default)]
    outcomes: Vec<OutcomeMetaEntry>,
    #[serde(default)]
    questions: Vec<OutcomeQuestionEntry>,
}

#[derive(Clone, Deserialize)]
struct OutcomeMetaEntry {
    outcome: u32,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "sideSpecs")]
    side_specs: Vec<OutcomeSideSpec>,
    #[serde(default = "default_outcome_quote_token", rename = "quoteToken")]
    quote_token: String,
}

#[derive(Clone, Deserialize)]
struct OutcomeSideSpec {
    name: String,
}

#[derive(Clone, Deserialize)]
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
    let questions_by_outcome = questions_by_outcome(&outcome_meta.questions);

    for outcome in outcome_meta.outcomes {
        let description_parts = parse_outcome_description(&outcome.description);
        let question = questions_by_outcome.get(&outcome.outcome);
        for (side_index, side) in outcome.side_specs.iter().enumerate() {
            if side_index > 1 {
                continue;
            }

            let side_index = side_index as u32;
            let quote_symbol = normalized_outcome_quote_symbol(&outcome.quote_token);
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
                quote_symbol.to_ascii_lowercase(),
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
                quote_symbol: quote_symbol.clone(),
                quote_token_index: outcome_quote_token_index(&quote_symbol),
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

fn default_outcome_quote_token() -> String {
    "USDC".to_string()
}

fn normalized_outcome_quote_symbol(raw: &str) -> String {
    let quote = raw.trim();
    if quote.is_empty() {
        default_outcome_quote_token()
    } else {
        quote.to_ascii_uppercase()
    }
}

fn outcome_quote_token_index(quote_symbol: &str) -> Option<u32> {
    match quote_symbol {
        "USDC" => Some(USDC_TOKEN_INDEX),
        "USDH" => Some(USDH_TOKEN_INDEX),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
