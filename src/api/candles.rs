use super::{API_URL, CLIENT, KEROSENE_USER_AGENT};
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::Serialize;

mod model;
mod normalize;
mod response;

#[cfg(test)]
mod tests;

pub use model::{Candle, de_string_or_number_to_f64};
pub use normalize::{is_valid_candle, normalize_candles};
use response::parse_candle_response;

#[derive(Debug, Clone, Serialize)]
struct CandleRequest {
    #[serde(rename = "type")]
    req_type: String,
    req: CandleRequestInner,
}

#[derive(Debug, Clone, Serialize)]
struct CandleRequestInner {
    coin: String,
    #[serde(rename = "startTime")]
    start_time: u64,
    #[serde(rename = "endTime")]
    end_time: u64,
    interval: String,
}

pub async fn fetch_candles(
    coin: String,
    interval: String,
    start_time: u64,
    end_time: u64,
) -> Result<Vec<Candle>, String> {
    let body = CandleRequest {
        req_type: "candleSnapshot".to_string(),
        req: CandleRequestInner {
            coin,
            start_time,
            end_time,
            interval,
        },
    };

    let client = CLIENT.clone();
    let response = client
        .post(API_URL)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    parse_candle_response(status, content_type.as_deref(), &text)
}
