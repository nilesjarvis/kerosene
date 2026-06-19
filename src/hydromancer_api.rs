use crate::{api::CLIENT, helpers::sensitive_response_snippet};
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Hydromancer REST API
// ---------------------------------------------------------------------------

const KEROSENE_USER_AGENT: &str = concat!("Kerosene/", env!("CARGO_PKG_VERSION"));
const FUNDING_HISTORY_PAGE_LIMIT: u16 = 500;
const FUNDING_HISTORY_MAX_PAGES: usize = 96;
// Reject absurd hourly funding rates before chart math can overflow.
const MAX_ABS_FUNDING_RATE: f64 = 1.0;

pub const HYDROMANCER_API_URL: &str = "https://api.hydromancer.xyz/info";

#[derive(Clone, PartialEq)]
pub(crate) struct FundingRatePoint {
    pub(crate) time_ms: u64,
    pub(crate) rate: f64,
}

impl fmt::Debug for FundingRatePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FundingRatePoint")
            .field("time_ms", &self.time_ms)
            .field("rate", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Serialize)]
struct FundingHistoryRequest<'a> {
    #[serde(rename = "type")]
    request_type: &'static str,
    coin: &'a str,
    #[serde(rename = "startTime")]
    start_time: u64,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    end_time: Option<u64>,
    limit: u16,
}

#[derive(Deserialize)]
struct RawFundingRatePoint {
    #[serde(rename = "fundingRate")]
    funding_rate: String,
    time: u64,
}

pub(crate) async fn fetch_funding_history(
    coin: String,
    start_time_ms: u64,
    end_time_ms: u64,
    api_key: Zeroizing<String>,
) -> Result<Vec<FundingRatePoint>, String> {
    if coin.trim().is_empty() {
        return Err("Hydromancer funding request missing coin".to_string());
    }
    if api_key.trim().is_empty() {
        return Err("Hydromancer API key is required for funding history".to_string());
    }
    if end_time_ms <= start_time_ms {
        return Ok(Vec::new());
    }

    let mut cursor = start_time_ms;
    let mut points = Vec::new();

    for _ in 0..FUNDING_HISTORY_MAX_PAGES {
        let page = fetch_funding_history_page(
            &coin,
            cursor,
            Some(end_time_ms),
            FUNDING_HISTORY_PAGE_LIMIT,
            api_key.as_str(),
        )
        .await?;
        if page.is_empty() {
            break;
        }

        let page_len = page.len();
        let Some(last_time) = page.last().map(|point| point.time_ms) else {
            break;
        };

        points.extend(
            page.into_iter()
                .filter(|point| point.time_ms >= start_time_ms && point.time_ms <= end_time_ms),
        );

        if last_time >= end_time_ms || page_len < FUNDING_HISTORY_PAGE_LIMIT as usize {
            break;
        }
        let next_cursor = last_time.saturating_add(1);
        if next_cursor <= cursor {
            break;
        }
        cursor = next_cursor;
    }

    points.sort_by_key(|point| point.time_ms);
    points.dedup_by_key(|point| point.time_ms);
    Ok(points)
}

async fn fetch_funding_history_page(
    coin: &str,
    start_time_ms: u64,
    end_time_ms: Option<u64>,
    limit: u16,
    api_key: &str,
) -> Result<Vec<FundingRatePoint>, String> {
    let body = FundingHistoryRequest {
        request_type: "fundingHistory",
        coin,
        start_time: start_time_ms,
        end_time: end_time_ms,
        limit,
    };

    let response = CLIENT
        .clone()
        .post(HYDROMANCER_API_URL)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Hydromancer funding request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Hydromancer funding response: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "Hydromancer funding HTTP {}: {}",
            status.as_u16(),
            sensitive_response_snippet(&text),
        ));
    }

    parse_funding_history_response(&text)
}

fn parse_funding_history_response(text: &str) -> Result<Vec<FundingRatePoint>, String> {
    let raw: Vec<RawFundingRatePoint> = serde_json::from_str(text).map_err(|e| {
        format!(
            "Hydromancer funding response parse failed: {e}; Response: {}",
            sensitive_response_snippet(text)
        )
    })?;
    normalize_funding_history(raw)
}

fn normalize_funding_history(
    raw: Vec<RawFundingRatePoint>,
) -> Result<Vec<FundingRatePoint>, String> {
    let mut points = Vec::with_capacity(raw.len());
    for point in raw {
        if point.time == 0 {
            return Err("Hydromancer funding response included missing timestamp".to_string());
        }
        let rate = point
            .funding_rate
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid Hydromancer funding rate: {e}"))?;
        if !rate.is_finite() {
            return Err("Hydromancer funding response included non-finite rate".to_string());
        }
        if rate.abs() > MAX_ABS_FUNDING_RATE {
            return Err(
                "Hydromancer funding response included unrealistic rate magnitude".to_string(),
            );
        }
        points.push(FundingRatePoint {
            time_ms: point.time,
            rate,
        });
    }
    points.sort_by_key(|point| point.time_ms);
    points.dedup_by_key(|point| point.time_ms);
    Ok(points)
}

#[cfg(test)]
mod tests;
