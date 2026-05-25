use super::OutcomeQuestionEntry;
use super::description::parse_outcome_description;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(super) struct OutcomeQuestionInfo {
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

pub(super) fn questions_by_outcome(
    questions: &[OutcomeQuestionEntry],
) -> HashMap<u32, OutcomeQuestionInfo> {
    let mut questions_by_outcome = HashMap::new();
    for question in questions {
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
    questions_by_outcome
}
