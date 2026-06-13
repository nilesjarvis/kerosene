use super::{API_URL, CLIENT, KEROSENE_USER_AGENT};
use crate::config::ChartBackfillSource;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::Serialize;
use zeroize::Zeroizing;

mod model;
mod normalize;
mod response;

#[cfg(test)]
mod tests;

pub use model::{Candle, de_string_or_number_to_f64};
pub use normalize::{is_valid_candle, normalize_candles};
use response::parse_candle_response;

const HYDROMANCER_API_URL: &str = "https://api.hydromancer.xyz/info";

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
    fetch_candles_from_endpoint(API_URL, None, coin, interval, start_time, end_time).await
}

pub async fn fetch_chart_backfill_candles(
    source: ChartBackfillSource,
    hydromancer_api_key: Zeroizing<String>,
    coin: String,
    interval: String,
    start_time: u64,
    end_time: u64,
) -> Result<Vec<Candle>, String> {
    match source {
        ChartBackfillSource::Hyperliquid => {
            fetch_candles_from_endpoint(API_URL, None, coin, interval, start_time, end_time).await
        }
        ChartBackfillSource::Hydromancer => {
            let api_key = Zeroizing::new(hydromancer_api_key.trim().to_string());
            if api_key.is_empty() {
                return fetch_candles_from_endpoint(
                    API_URL, None, coin, interval, start_time, end_time,
                )
                .await;
            }

            let interval_ms = candle_interval_ms(&interval);
            let hydromancer_result = fetch_candles_from_endpoint(
                HYDROMANCER_API_URL,
                Some(api_key),
                coin.clone(),
                interval.clone(),
                start_time,
                end_time,
            )
            .await;
            let candles = match hydromancer_result {
                Ok(candles) => candles,
                Err(hydromancer_error) => {
                    return fetch_candles_from_endpoint(
                        API_URL,
                        None,
                        coin,
                        interval,
                        start_time,
                        end_time,
                    )
                    .await
                    .map_err(|fallback_error| {
                        format!(
                            "Hydromancer chart backfill failed: {hydromancer_error}; Hyperliquid fallback failed: {fallback_error}"
                        )
                    });
                }
            };
            Ok(match interval_ms {
                Some(interval_ms) => fill_zero_volume_candle_gaps(candles, interval_ms),
                None => candles,
            })
        }
    }
}

async fn fetch_candles_from_endpoint(
    url: &str,
    bearer_token: Option<Zeroizing<String>>,
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

    let redact_sensitive_response = bearer_token.is_some();
    let client = CLIENT.clone();
    let mut request = client
        .post(url)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .json(&body);
    if let Some(token) = bearer_token {
        request = request.bearer_auth(token.as_str());
    }

    let response = request
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

    parse_candle_response(
        status,
        content_type.as_deref(),
        &text,
        redact_sensitive_response,
    )
}

fn candle_interval_ms(interval: &str) -> Option<u64> {
    Some(match interval {
        "1m" => 60_000,
        "3m" => 3 * 60_000,
        "5m" => 5 * 60_000,
        "15m" => 15 * 60_000,
        "30m" => 30 * 60_000,
        "1h" => 60 * 60_000,
        "2h" => 2 * 60 * 60_000,
        "4h" => 4 * 60 * 60_000,
        "8h" => 8 * 60 * 60_000,
        "12h" => 12 * 60 * 60_000,
        "1d" => 24 * 60 * 60_000,
        "3d" => 3 * 24 * 60 * 60_000,
        "1w" => 7 * 24 * 60 * 60_000,
        _ => return None,
    })
}

fn fill_zero_volume_candle_gaps(candles: Vec<Candle>, interval_ms: u64) -> Vec<Candle> {
    if candles.len() < 2 || interval_ms == 0 {
        return candles;
    }

    let mut filled: Vec<Candle> = Vec::with_capacity(candles.len());
    for candle in candles {
        while let Some(previous) = filled.last() {
            let next_time = previous.open_time.saturating_add(interval_ms);
            if next_time >= candle.open_time {
                break;
            }
            filled.push(Candle {
                open_time: next_time,
                close_time: next_time.saturating_add(interval_ms).saturating_sub(1),
                open: previous.close,
                high: previous.close,
                low: previous.close,
                close: previous.close,
                volume: 0.0,
            });
        }
        filled.push(candle);
    }

    filled
}
