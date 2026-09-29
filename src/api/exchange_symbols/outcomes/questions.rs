use super::OutcomeQuestionEntry;
use super::description::parse_outcome_description;
use super::templates::{OutcomeTemplate, parse_parameters, resolve_template};
use crate::api::OutcomeContract;

use std::collections::HashMap;

pub(super) struct OutcomeQuestionInfo {
    pub(super) contract: OutcomeContract,
    pub(super) template_id: Option<String>,
    pub(super) question_id: u32,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) class: Option<String>,
    pub(super) underlying: Option<String>,
    pub(super) expiry: Option<String>,
    pub(super) price_thresholds: Vec<String>,
    pub(super) period: Option<String>,
    pub(super) named_outcomes: Vec<u32>,
    pub(super) settled_named_outcomes: Vec<u32>,
    pub(super) fallback_outcome: Option<u32>,
}

impl OutcomeQuestionInfo {
    pub(super) fn from_entry(entry: OutcomeQuestionEntry, templates: &[OutcomeTemplate]) -> Self {
        let mut description_parts = parse_outcome_description(&entry.description);
        let price_thresholds = description_parts
            .get("priceThresholds")
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let template_id = entry.name.strip_prefix("template:").map(str::to_string);
        let contract = if template_id.is_some() {
            resolve_template(&entry.name, &entry.description, templates)
                .and_then(|resolved| {
                    if resolved.is_question {
                        Ok(resolved.contract)
                    } else {
                        Err("Outcome template used as a question".into())
                    }
                })
                .unwrap_or_else(|reason| OutcomeContract {
                    title: Some(format!("Question {} — details unavailable", entry.question)),
                    blocked_reason: Some(reason),
                    ..OutcomeContract::default()
                })
        } else {
            OutcomeContract {
                blocked_reason: parse_parameters(&entry.description).err(),
                ..OutcomeContract::default()
            }
        };
        Self {
            question_id: entry.question,
            name: contract.title.clone().unwrap_or(entry.name),
            contract,
            template_id,
            description: entry.description,
            class: description_parts.remove("class"),
            underlying: description_parts.remove("underlying"),
            expiry: description_parts.remove("expiry"),
            price_thresholds,
            period: description_parts.remove("period"),
            named_outcomes: entry.named_outcomes,
            settled_named_outcomes: entry.settled_named_outcomes,
            fallback_outcome: entry.fallback_outcome,
        }
    }
}

pub(super) fn questions_by_outcome(
    questions: &[OutcomeQuestionInfo],
) -> HashMap<u32, &OutcomeQuestionInfo> {
    let mut questions_by_outcome = HashMap::new();
    for question in questions {
        for outcome_id in question
            .named_outcomes
            .iter()
            .chain(question.settled_named_outcomes.iter())
            .copied()
            .chain(question.fallback_outcome)
        {
            questions_by_outcome.entry(outcome_id).or_insert(question);
        }
    }
    questions_by_outcome
}
