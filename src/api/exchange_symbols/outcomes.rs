mod contracts;
mod description;
mod encoding;
mod questions;
pub(super) mod templates;

use super::model::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::api::{USDC_TOKEN_INDEX, USDH_TOKEN_INDEX};

use contracts::resolve_contract;
use description::parse_outcome_description;
use encoding::{outcome_asset_index, outcome_coin_key, outcome_encoding};
use questions::questions_by_outcome;
use serde::Deserialize;
use templates::OutcomeTemplate;

#[derive(Clone, Deserialize)]
pub(super) struct OutcomeMetaResponse {
    outcomes: Vec<OutcomeMetaEntry>,
    #[serde(default)]
    questions: Vec<OutcomeQuestionEntry>,
    #[serde(default, rename = "feeScale")]
    fee_scale: Option<String>,
}

#[derive(Clone, Deserialize)]
struct OutcomeMetaEntry {
    outcome: u32,
    #[serde(default)]
    venue: Option<String>,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "sideSpecs")]
    side_specs: Vec<OutcomeSideSpec>,
    #[serde(default = "default_outcome_quote_token", rename = "quoteToken")]
    quote_token: String,
    #[serde(default, rename = "deployerFeeScale")]
    deployer_fee_scale: Option<String>,
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

pub(super) fn parse_outcome_symbols(
    outcome_meta: OutcomeMetaResponse,
    templates: &[OutcomeTemplate],
) -> Result<Vec<ExchangeSymbol>, String> {
    validate_metadata(&outcome_meta)?;
    let questions_by_outcome = questions_by_outcome(&outcome_meta.questions, templates);
    let mut symbols = Vec::new();

    for outcome in outcome_meta.outcomes {
        let mut description_parts = parse_outcome_description(&outcome.description);
        // Current Skew contracts (and other HIP-4 deployers) use the chain's
        // binaryPrice template. Normalize its fields into the existing binary
        // model, retaining the original name/description for settlement details.
        if outcome.name == "template:binaryPrice" {
            description_parts.insert("class".to_string(), "priceBinary".to_string());
            for (source, target) in [
                ("perp", "underlying"),
                ("threshold", "targetPrice"),
                ("time", "expiry"),
            ] {
                if let Some(value) = description_parts.get(source).cloned() {
                    description_parts.insert(target.to_string(), value);
                }
            }
        }
        let venue = outcome
            .venue
            .as_deref()
            .map(str::trim)
            .filter(|venue| !venue.is_empty())
            .map(str::to_ascii_lowercase);
        let question = questions_by_outcome.get(&outcome.outcome);
        let (mut contract, side_names) = resolve_contract(&outcome, question, templates);
        contract.fee_scale = valid_fee_scale(outcome_meta.fee_scale.as_deref());
        contract.deployer_fee_scale = valid_fee_scale(outcome.deployer_fee_scale.as_deref());
        for side_index in 0..2 {
            let side_index = side_index as u32;
            let quote_symbol = normalized_outcome_quote_symbol(&outcome.quote_token);
            let side_name = side_names[side_index as usize].clone();
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
            if let Some(venue) = &venue {
                keywords.push(venue.clone());
                keywords.push(OutcomeSymbolInfo::venue_display_name(venue).to_lowercase());
                if venue == "skew" {
                    keywords.push("skew.trade".to_string());
                }
            }
            let info = OutcomeSymbolInfo {
                outcome_id: outcome.outcome,
                contract: contract.clone(),
                venue: venue.clone(),
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
                growth_mode: false,
                market_type: MarketType::Outcome,
                outcome: Some(info),
            });
        }
    }
    Ok(symbols)
}

fn validate_metadata(meta: &OutcomeMetaResponse) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for outcome in &meta.outcomes {
        if !ids.insert(outcome.outcome)
            || outcome
                .outcome
                .checked_mul(10)
                .and_then(|encoding| encoding.checked_add(1))
                .and_then(|encoding| encoding.checked_add(crate::api::OUTCOME_ASSET_ID_OFFSET))
                .is_none()
        {
            return Err("Invalid or duplicate outcome ID".into());
        }
        if outcome.side_specs.len() != 2 {
            return Err("Outcome metadata must contain exactly two sides".into());
        }
    }
    let mut questions = std::collections::HashSet::new();
    let mut members = std::collections::HashMap::new();
    for question in &meta.questions {
        if !questions.insert(question.question) {
            return Err("Duplicate outcome question ID".into());
        }
        for outcome in question
            .named_outcomes
            .iter()
            .chain(&question.settled_named_outcomes)
            .copied()
            .chain(question.fallback_outcome)
        {
            if members
                .insert(outcome, question.question)
                .is_some_and(|previous| previous != question.question)
            {
                return Err("Outcome belongs to multiple questions".into());
            }
        }
        if question.fallback_outcome.is_some_and(|fallback| {
            question.named_outcomes.contains(&fallback)
                || question.settled_named_outcomes.contains(&fallback)
        }) {
            return Err("Fallback outcome is also a named outcome".into());
        }
    }
    Ok(())
}

fn valid_fee_scale(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    raw.parse::<f64>()
        .is_ok_and(|value| value.is_finite() && value >= 0.0)
        .then(|| raw.to_string())
}

#[cfg(test)]
fn append_outcome_symbols(symbols: &mut Vec<ExchangeSymbol>, meta: OutcomeMetaResponse) {
    let templates = serde_json::from_str::<Vec<OutcomeTemplate>>(include_str!(
        "outcomes/tests/fixtures/templates.json"
    ))
    .expect("template fixture");
    symbols.extend(parse_outcome_symbols(meta, &templates).expect("valid outcome fixture"));
}

fn default_outcome_quote_token() -> String {
    "UNKNOWN".to_string()
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
