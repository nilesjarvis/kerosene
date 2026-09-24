use crate::api::OutcomeContract;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
pub(crate) struct OutcomeTemplate {
    id: String,
    role: TemplateRole,
    name: String,
    description: String,
    keywords: Vec<(String, String)>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TemplateRole {
    Question,
    StandaloneOutcome {
        #[serde(rename = "sideNames")]
        side_names: [String; 2],
    },
    QuestionOutcome {
        parent: String,
    },
}

pub(super) struct ResolvedTemplate {
    pub contract: OutcomeContract,
    pub sides: Option<[String; 2]>,
    pub parent: Option<String>,
    pub is_question: bool,
}

pub(super) fn resolve_template(
    name: &str,
    description: &str,
    templates: &[OutcomeTemplate],
) -> Result<ResolvedTemplate, String> {
    let id = name
        .strip_prefix("template:")
        .ok_or("Missing template ID")?;
    let mut matches = templates.iter().filter(|template| template.id == id);
    let template = matches.next().ok_or("Contract template is unavailable")?;
    if matches.next().is_some() {
        return Err("Duplicate contract template".into());
    }
    let values = parse_parameters(description)?;
    let mut display_values = HashMap::new();
    for (key, kind) in &template.keywords {
        let value = *values
            .get(key.as_str())
            .ok_or("Missing contract parameter")?;
        if value.is_empty() {
            return Err("Empty contract parameter".into());
        }
        let display = match kind.as_str() {
            "dateTime" => {
                let timestamp = parse_deadline(value).ok_or("Invalid contract date")?;
                let date = chrono::DateTime::from_timestamp_millis(timestamp as i64)
                    .ok_or("Invalid contract date")?;
                date.format("%Y-%m-%d %H:%M UTC").to_string()
            }
            "uInt" if value.parse::<u64>().is_ok() => value.to_string(),
            "uDecimal" if valid_unsigned_decimal(value) => value.to_string(),
            "string" | "shortString" | "hlPerp" => value.to_string(),
            _ => return Err("Invalid or unsupported contract parameter type".into()),
        };
        if display_values.insert(key.as_str(), display).is_some() {
            return Err("Duplicate template parameter".into());
        }
    }
    let (deadline_key, resolution_deadline) = match id {
        "binaryPrice" | "priceTouch" | "scalarPrice" => (Some("time"), false),
        "companyIpoConfirmed" | "aiModelHeadToHead" => (Some("dateTime"), false),
        "sportsContestWinner"
        | "sportsContestResult"
        | "sportsTournamentWinner"
        | "sportsOverUnderMarket"
        | "sportsScalarMarket" => (Some("resolutionDeadline"), true),
        "policyRateDecision" => (Some("decisionDeadline"), true),
        _ if matches!(template.role, TemplateRole::QuestionOutcome { .. }) => (None, true),
        _ => return Err("Contract lifecycle is not supported".into()),
    };
    let deadline_ms = deadline_key
        .map(|key| {
            values
                .get(key)
                .and_then(|value| parse_deadline(value))
                .ok_or("Invalid contract deadline")
        })
        .transpose()?;
    let scalar = matches!(id, "scalarPrice" | "sportsScalarMarket");
    if scalar {
        let low = values
            .get("low")
            .and_then(|value| value.parse::<f64>().ok());
        let high = values
            .get("high")
            .and_then(|value| value.parse::<f64>().ok());
        if !matches!((low, high), (Some(low), Some(high)) if low < high) {
            return Err("Invalid scalar payout range".into());
        }
    }
    if values
        .get("seconds")
        .is_some_and(|value| value.parse::<u64>().ok() == Some(0))
    {
        return Err("Invalid settlement window".into());
    }
    let (sides, parent, is_question) = match &template.role {
        TemplateRole::Question => (None, None, true),
        TemplateRole::QuestionOutcome { parent } => (None, Some(parent.clone()), false),
        TemplateRole::StandaloneOutcome { side_names } => (
            Some([
                interpolate(&side_names[0], &display_values)?,
                interpolate(&side_names[1], &display_values)?,
            ]),
            None,
            false,
        ),
    };
    Ok(ResolvedTemplate {
        contract: OutcomeContract {
            verified: true,
            title: Some(interpolate(&template.name, &display_values)?),
            rules: Some(interpolate(
                template
                    .description
                    .split(" metadata=")
                    .next()
                    .unwrap_or(&template.description),
                &display_values,
            )?),
            deadline_ms,
            resolution_deadline,
            scalar,
            ..OutcomeContract::default()
        },
        sides,
        parent,
        is_question,
    })
}

/// Substitute once: parameter text cannot inject another template placeholder.
fn interpolate(template: &str, values: &HashMap<&str, String>) -> Result<String, String> {
    let mut remaining = template;
    let mut rendered = String::new();
    while let Some(start) = remaining.find('{') {
        rendered.push_str(&remaining[..start]);
        let rest = &remaining[start + 1..];
        let end = rest.find('}').ok_or("Malformed contract template")?;
        rendered.push_str(
            values
                .get(&rest[..end])
                .ok_or("Unresolved contract placeholder")?,
        );
        remaining = &rest[end + 1..];
    }
    rendered.push_str(remaining);
    if rendered.trim().is_empty() {
        return Err("Empty contract label or rules".into());
    }
    Ok(rendered.replace(" UTC UTC", " UTC"))
}

pub(super) fn parse_deadline(value: &str) -> Option<u64> {
    if value.len() != 13
        || !value.bytes().enumerate().all(|(i, c)| {
            if i == 8 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
    {
        return None;
    }
    let date = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%d-%H%M").ok()?;
    u64::try_from(date.and_utc().timestamp_millis()).ok()
}

pub(super) fn parse_parameters(description: &str) -> Result<HashMap<&str, &str>, String> {
    let mut values = HashMap::new();
    for part in description
        .split('|')
        .filter(|part| !part.trim().is_empty())
    {
        let (key, value) = part
            .split_once(':')
            .ok_or("Malformed contract parameters")?;
        if key.trim().is_empty()
            || value.trim().is_empty()
            || values.insert(key.trim(), value.trim()).is_some()
        {
            return Err("Duplicate or empty contract parameter".into());
        }
    }
    Ok(values)
}

pub(super) fn resolve_side_name(name: &str, description: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Empty outcome side label".into());
    }
    let Some(template) = name.strip_prefix("template:") else {
        return Ok(name.to_string());
    };
    let values = description
        .split('|')
        .filter_map(|part| part.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim().to_string()))
        .collect();
    interpolate(template, &values)
}

fn valid_unsigned_decimal(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_digit() || c == '.')
        && value
            .parse::<f64>()
            .is_ok_and(|value| value.is_finite() && value >= 0.0)
}

#[cfg(test)]
mod tests;
