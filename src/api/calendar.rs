use super::{CLIENT, KEROSENE_USER_AGENT};
use crate::helpers::response_excerpt;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CalendarEvent {
    pub title: String,
    pub country: String,
    pub date: String,
    pub impact: String,
    pub forecast: String,
    pub previous: String,
}

pub async fn fetch_economic_calendar() -> Result<Vec<CalendarEvent>, String> {
    let url = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";
    let response = CLIENT
        .clone()
        .get(url)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Economic calendar request failed: {e}"))?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read economic calendar response: {e}"))?;

    let snippet = || response_excerpt(&text);

    if !status.is_success() {
        return Err(format!(
            "Economic calendar request failed (HTTP {}): {}",
            status,
            snippet()
        ));
    }

    if content_type
        .as_deref()
        .is_some_and(|value| !value.to_ascii_lowercase().contains("json"))
        && !text.trim_start().starts_with('[')
    {
        return Err(format!(
            "Economic calendar returned {} instead of JSON: {}",
            content_type.unwrap_or_else(|| "unknown content".to_string()),
            snippet()
        ));
    }

    let mut events: Vec<CalendarEvent> = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse economic calendar JSON: {e}; {}", snippet()))?;
    events.sort_by(|a, b| {
        let a_ts = chrono::DateTime::parse_from_rfc3339(&a.date)
            .map(|dt| dt.timestamp())
            .ok();
        let b_ts = chrono::DateTime::parse_from_rfc3339(&b.date)
            .map(|dt| dt.timestamp())
            .ok();
        a_ts.cmp(&b_ts).then_with(|| a.date.cmp(&b.date))
    });

    Ok(events)
}
