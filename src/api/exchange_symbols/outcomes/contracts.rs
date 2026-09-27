use super::OutcomeMetaEntry;
use super::description::parse_outcome_description;
use super::questions::OutcomeQuestionInfo;
use super::templates::{
    OutcomeTemplate, parse_deadline, parse_parameters, resolve_side_name, resolve_template,
};
use crate::api::OutcomeContract;

pub(super) fn resolve_contract(
    outcome: &OutcomeMetaEntry,
    question: Option<&OutcomeQuestionInfo>,
    templates: &[OutcomeTemplate],
) -> (OutcomeContract, [String; 2]) {
    let sides = outcome
        .side_specs
        .iter()
        .map(|side| resolve_side_name(&side.name, &outcome.description))
        .collect::<Result<Vec<_>, _>>();
    let side_names = sides
        .as_ref()
        .ok()
        .and_then(|sides| <[String; 2]>::try_from(sides.clone()).ok())
        .unwrap_or_else(|| ["Side 0".to_string(), "Side 1".to_string()]);
    let resolved = || -> Result<OutcomeContract, String> {
        sides.as_ref().map_err(Clone::clone)?;
        if side_names[0] == side_names[1] {
            return Err("Outcome sides must have distinct labels".into());
        }
        if outcome.name.starts_with("template:") {
            let resolved = resolve_template(&outcome.name, &outcome.description, templates)?;
            if resolved.is_question {
                return Err("Question template used as an outcome".into());
            }
            let mut contract = resolved.contract;
            if let Some(parent) = resolved.parent {
                let question = question.ok_or("Parent question is unavailable")?;
                if question.template_id.as_deref() != Some(parent.as_str()) {
                    return Err("Outcome parent template does not match its question".into());
                }
                if let Some(reason) = &question.contract.blocked_reason {
                    return Err(reason.clone());
                }
                if side_names != ["Yes", "No"] {
                    return Err("Question outcome sides do not match Yes/No".into());
                }
                contract.deadline_ms = question.contract.deadline_ms;
                contract.resolution_deadline = question.contract.resolution_deadline;
                contract.rules = Some(format!(
                    "{}\n\n{}",
                    question.contract.rules.as_deref().unwrap_or(""),
                    contract.rules.as_deref().unwrap_or("")
                ));
            } else if question.is_some() || resolved.sides.as_ref() != Some(&side_names) {
                return Err("Outcome sides or question do not match its template".into());
            }
            Ok(contract)
        } else {
            parse_parameters(&outcome.description)?;
            if let Some(reason) =
                question.and_then(|question| question.contract.blocked_reason.as_deref())
            {
                return Err(reason.to_string());
            }
            let parts = parse_outcome_description(&outcome.description);
            let is_binary = parts
                .get("class")
                .is_some_and(|class| class == "priceBinary");
            let is_bucket =
                question.is_some_and(|question| question.class.as_deref() == Some("priceBucket"));
            let expiry = if is_bucket {
                question.and_then(|question| question.expiry.as_deref())
            } else {
                parts.get("expiry").map(String::as_str)
            };
            let deadline_ms = expiry
                .and_then(parse_deadline)
                .ok_or("Contract deadline is unavailable")?;
            if !is_binary && !is_bucket {
                return Err("Contract rules are not supported".into());
            }
            if side_names != ["Yes", "No"] {
                return Err("Binary outcome sides do not match Yes/No".into());
            }
            if is_binary
                && (!parts.contains_key("underlying")
                    || !parts.get("targetPrice").is_some_and(|value| {
                        value
                            .parse::<f64>()
                            .is_ok_and(|price| price.is_finite() && price > 0.0)
                    }))
            {
                return Err("Invalid binary price contract".into());
            }
            if is_bucket {
                let question = question.ok_or("Missing bucket question")?;
                let index = parts
                    .get("index")
                    .and_then(|index| index.parse::<usize>().ok());
                if question.underlying.is_none()
                    || question.price_thresholds.is_empty()
                    || index.is_none_or(|index| index > question.price_thresholds.len())
                {
                    return Err("Invalid price bucket contract".into());
                }
                let prices = question
                    .price_thresholds
                    .iter()
                    .map(|price| price.parse::<f64>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "Invalid bucket thresholds")?;
                if prices
                    .iter()
                    .any(|price| !price.is_finite() || *price <= 0.0)
                    || prices.windows(2).any(|pair| pair[0] >= pair[1])
                {
                    return Err("Invalid bucket thresholds".into());
                }
            }
            Ok(OutcomeContract {
                verified: true,
                deadline_ms: Some(deadline_ms),
                ..OutcomeContract::default()
            })
        }
    };
    let contract = resolved().unwrap_or_else(|reason| OutcomeContract {
        verified: true,
        title: outcome
            .name
            .starts_with("template:")
            .then(|| format!("Outcome {} — details unavailable", outcome.outcome)),
        blocked_reason: Some(reason),
        ..OutcomeContract::default()
    });
    (contract, side_names)
}
