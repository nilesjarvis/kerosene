use crate::api::CLIENT;
use crate::helpers::text_excerpt;
use crate::x_feed::{
    XFeedStreamEvent, build_x_feed_query, normalize_x_bearer_token_input, normalized_x_handle_list,
    parse_x_stream_page, x_api_auth_guidance,
};
use futures::{SinkExt as _, StreamExt as _, channel::mpsc};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

const X_RULES_ENDPOINT: &str = "https://api.x.com/2/tweets/search/stream/rules";
const X_STREAM_ENDPOINT: &str = "https://api.x.com/2/tweets/search/stream";
const X_RULE_TAG: &str = "kerosene:x-feed";
const X_STREAM_RECONNECT_BASE_DELAY: Duration = Duration::from_secs(2);
const X_STREAM_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);
const X_STREAM_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct XFeedStreamParams {
    pub(crate) bearer_token: String,
    pub(crate) handles: Vec<String>,
    pub(crate) reconnect_nonce: u64,
}

pub(crate) fn x_feed_stream(
    params: &XFeedStreamParams,
) -> Pin<Box<dyn futures::Stream<Item = XFeedStreamEvent> + Send>> {
    let params = params.clone();
    Box::pin(iced::stream::channel(1000, async move |mut output| {
        let mut retry_delay = X_STREAM_RECONNECT_BASE_DELAY;
        loop {
            match run_x_feed_stream_session(&params, &mut output).await {
                XStreamSessionExit::Retry => {
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = next_x_reconnect_delay(retry_delay);
                }
                XStreamSessionExit::Stop => return,
            }
        }
    }))
}

enum XStreamSessionExit {
    Retry,
    Stop,
}

async fn run_x_feed_stream_session(
    params: &XFeedStreamParams,
    output: &mut mpsc::Sender<XFeedStreamEvent>,
) -> XStreamSessionExit {
    let bearer_token = normalize_x_bearer_token_input(&params.bearer_token);
    if bearer_token.is_empty() {
        let _ = send_status(output, false, "Enter an X API bearer token").await;
        return XStreamSessionExit::Stop;
    }

    let handles = normalized_x_handle_list(&params.handles);
    if handles.is_empty() {
        let _ = send_status(output, false, "Add a public X handle").await;
        return XStreamSessionExit::Stop;
    }

    let query = match build_x_feed_query(&handles) {
        Ok(query) => query,
        Err(err) => {
            let _ = send_status(output, false, &err).await;
            return XStreamSessionExit::Stop;
        }
    };

    if let Err(err) = sync_x_stream_rules(&bearer_token, &query).await {
        let _ = send_status(output, false, &err).await;
        return XStreamSessionExit::Retry;
    }

    if !send_status(output, true, "X stream connected").await {
        return XStreamSessionExit::Stop;
    }

    let response = match CLIENT
        .get(X_STREAM_ENDPOINT)
        .bearer_auth(&bearer_token)
        .query(&[
            (
                "tweet.fields",
                "created_at,author_id,entities,public_metrics",
            ),
            ("expansions", "author_id"),
            ("user.fields", "username,name,profile_image_url,verified"),
        ])
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            let _ = send_status(output, false, &format!("X stream request failed: {err}")).await;
            return XStreamSessionExit::Retry;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let preview = text_excerpt(&body, 160);
        let message = if preview.is_empty() {
            format!("X stream failed with HTTP {status}")
        } else {
            format!("X stream failed with HTTP {status}: {preview}")
        };
        let _ = send_status(output, false, &message).await;
        return XStreamSessionExit::Retry;
    }

    let mut chunks = response.bytes_stream();
    let mut buffer = Vec::new();
    loop {
        match tokio::time::timeout(X_STREAM_KEEPALIVE_TIMEOUT, chunks.next()).await {
            Ok(Some(Ok(chunk))) => {
                buffer.extend_from_slice(&chunk);
                while let Some(line_end) = buffer.iter().position(|byte| *byte == b'\n') {
                    let line = buffer.drain(..=line_end).collect::<Vec<_>>();
                    let line = line
                        .strip_suffix(b"\n")
                        .unwrap_or(&line)
                        .strip_suffix(b"\r")
                        .unwrap_or(line.as_slice());
                    if line.iter().all(|byte| byte.is_ascii_whitespace()) {
                        continue;
                    }
                    let fetched_at_ms = system_time_ms();
                    match parse_x_stream_page(line, fetched_at_ms) {
                        Ok(page) if !page.posts.is_empty() => {
                            if output
                                .send(XFeedStreamEvent::Loaded(Box::new(Ok(page))))
                                .await
                                .is_err()
                            {
                                return XStreamSessionExit::Stop;
                            }
                        }
                        Ok(_) => {}
                        Err(err) => {
                            let _ = output
                                .send(XFeedStreamEvent::Loaded(Box::new(Err(err))))
                                .await;
                        }
                    }
                }
            }
            Ok(Some(Err(err))) => {
                let _ = send_status(output, false, &format!("X stream disconnected: {err}")).await;
                return XStreamSessionExit::Retry;
            }
            Ok(None) => {
                let _ = send_status(output, false, "X stream closed").await;
                return XStreamSessionExit::Retry;
            }
            Err(_) => {
                let _ =
                    send_status(output, false, "X stream timed out waiting for keepalive").await;
                return XStreamSessionExit::Retry;
            }
        }
    }
}

async fn sync_x_stream_rules(bearer_token: &str, query: &str) -> Result<(), String> {
    let existing = fetch_x_stream_rules(bearer_token).await?;
    let owned = existing
        .iter()
        .filter(|rule| rule.tag.as_deref() == Some(X_RULE_TAG))
        .collect::<Vec<_>>();
    if owned.len() == 1 && owned[0].value == query {
        return Ok(());
    }

    let delete_ids = owned.iter().map(|rule| rule.id.clone()).collect::<Vec<_>>();
    if !delete_ids.is_empty() {
        delete_x_stream_rules(bearer_token, delete_ids).await?;
    }

    add_x_stream_rule(bearer_token, query).await
}

async fn fetch_x_stream_rules(bearer_token: &str) -> Result<Vec<XRule>, String> {
    let response = CLIENT
        .get(X_RULES_ENDPOINT)
        .bearer_auth(bearer_token)
        .send()
        .await
        .map_err(|e| format!("X stream rules request failed: {e}"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("X stream rules response read failed: {e}"))?;
    if !status.is_success() {
        return Err(http_error("X stream rules request", status, &body));
    }
    let response: XRulesResponse = serde_json::from_slice(&body)
        .map_err(|e| format!("X stream rules response parse failed: {e}"))?;
    Ok(response.data.unwrap_or_default())
}

async fn delete_x_stream_rules(bearer_token: &str, ids: Vec<String>) -> Result<(), String> {
    let response = CLIENT
        .post(X_RULES_ENDPOINT)
        .bearer_auth(bearer_token)
        .json(&XRulesDeleteRequest {
            delete: XRulesDelete { ids },
        })
        .send()
        .await
        .map_err(|e| format!("X stream rule delete failed: {e}"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("X stream rule delete response read failed: {e}"))?;
    if status.is_success() {
        Ok(())
    } else {
        Err(http_error("X stream rule delete", status, &body))
    }
}

async fn add_x_stream_rule(bearer_token: &str, query: &str) -> Result<(), String> {
    let response = CLIENT
        .post(X_RULES_ENDPOINT)
        .bearer_auth(bearer_token)
        .json(&XRulesAddRequest {
            add: vec![XRuleAdd {
                value: query.to_string(),
                tag: X_RULE_TAG.to_string(),
            }],
        })
        .send()
        .await
        .map_err(|e| format!("X stream rule add failed: {e}"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("X stream rule add response read failed: {e}"))?;
    if status.is_success() {
        Ok(())
    } else {
        Err(http_error("X stream rule add", status, &body))
    }
}

fn http_error(prefix: &str, status: reqwest::StatusCode, body: &[u8]) -> String {
    let preview = String::from_utf8_lossy(body)
        .chars()
        .take(160)
        .collect::<String>();
    if let Some(message) = x_api_auth_guidance(&preview) {
        message
    } else if preview.is_empty() {
        format!("{prefix} failed with HTTP {status}")
    } else {
        format!("{prefix} failed with HTTP {status}: {preview}")
    }
}

async fn send_status(
    output: &mut mpsc::Sender<XFeedStreamEvent>,
    connected: bool,
    message: &str,
) -> bool {
    output
        .send(XFeedStreamEvent::Status {
            connected,
            message: message.to_string(),
        })
        .await
        .is_ok()
}

fn next_x_reconnect_delay(current: Duration) -> Duration {
    let next = current.saturating_mul(2);
    if next > X_STREAM_RECONNECT_MAX_DELAY {
        X_STREAM_RECONNECT_MAX_DELAY
    } else {
        next
    }
}

fn system_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Deserialize)]
struct XRulesResponse {
    data: Option<Vec<XRule>>,
}

#[derive(Debug, Deserialize)]
struct XRule {
    id: String,
    value: String,
    tag: Option<String>,
}

#[derive(Debug, Serialize)]
struct XRulesAddRequest {
    add: Vec<XRuleAdd>,
}

#[derive(Debug, Serialize)]
struct XRuleAdd {
    value: String,
    tag: String,
}

#[derive(Debug, Serialize)]
struct XRulesDeleteRequest {
    delete: XRulesDelete,
}

#[derive(Debug, Serialize)]
struct XRulesDelete {
    ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_delay_is_capped() {
        assert_eq!(
            next_x_reconnect_delay(Duration::from_secs(40)),
            X_STREAM_RECONNECT_MAX_DELAY
        );
    }
}
