use crate::api::{CLIENT, KEROSENE_USER_AGENT};
use crate::app_time::now_ms;
use crate::helpers::sensitive_response_excerpt;
use crate::x_feed::{
    XFeedStreamEvent, build_x_feed_query, normalize_x_bearer_token_input, normalized_x_handle_list,
    parse_x_stream_page, x_api_auth_guidance,
};
use futures::{SinkExt as _, StreamExt as _, channel::mpsc};
use iced::Subscription;
use iced::advanced::subscription::{EventStream, Hasher, Recipe, from_recipe};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::fmt;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::OnceLock;
use std::time::Duration;
use zeroize::Zeroizing;

const X_RULES_ENDPOINT: &str = "https://api.x.com/2/tweets/search/stream/rules";
const X_STREAM_ENDPOINT: &str = "https://api.x.com/2/tweets/search/stream";
const X_RULE_TAG: &str = "kerosene:x-feed";
const X_RULE_TAG_PREFIX: &str = "kerosene:x-feed:";
const X_STREAM_RECONNECT_BASE_DELAY: Duration = Duration::from_secs(2);
const X_STREAM_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);
const X_STREAM_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(45);
const X_STREAM_MAX_LINE_BYTES: usize = 256 * 1024;
const X_STREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const X_STREAM_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XStreamHttpClientPolicy {
    connect_timeout: Duration,
    total_timeout: Option<Duration>,
    pool_idle_timeout: Duration,
}

const X_STREAM_HTTP_CLIENT_POLICY: XStreamHttpClientPolicy = XStreamHttpClientPolicy {
    connect_timeout: X_STREAM_CONNECT_TIMEOUT,
    total_timeout: None,
    pool_idle_timeout: X_STREAM_POOL_IDLE_TIMEOUT,
};

fn build_x_stream_client(policy: XStreamHttpClientPolicy) -> Result<Client, reqwest::Error> {
    let mut headers = HeaderMap::new();
    if let Ok(user_agent) = HeaderValue::from_str(KEROSENE_USER_AGENT) {
        headers.insert(USER_AGENT, user_agent);
    }

    let mut builder = Client::builder()
        .default_headers(headers)
        .connect_timeout(policy.connect_timeout)
        .pool_idle_timeout(policy.pool_idle_timeout);
    if let Some(total_timeout) = policy.total_timeout {
        builder = builder.timeout(total_timeout);
    }
    builder.build()
}

fn x_stream_client() -> &'static Client {
    static X_STREAM_CLIENT: OnceLock<Client> = OnceLock::new();
    X_STREAM_CLIENT.get_or_init(|| {
        build_x_stream_client(X_STREAM_HTTP_CLIENT_POLICY).unwrap_or_else(|_| Client::new())
    })
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XFeedStreamParams {
    pub(crate) bearer_token: Zeroizing<String>,
    pub(crate) handles: Vec<String>,
    pub(crate) reconnect_nonce: u64,
}

impl fmt::Debug for XFeedStreamParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedStreamParams")
            .field("bearer_token", &"<redacted>")
            .field("handles", &self.handles)
            .field("reconnect_nonce", &self.reconnect_nonce)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct XFeedStreamIdentity {
    handles: Vec<String>,
    reconnect_nonce: u64,
}

impl XFeedStreamIdentity {
    fn for_params(params: &XFeedStreamParams) -> Self {
        Self {
            handles: normalized_x_handle_list(&params.handles),
            reconnect_nonce: params.reconnect_nonce,
        }
    }
}

struct XFeedStreamRecipe {
    identity: XFeedStreamIdentity,
    params: XFeedStreamParams,
}

impl Recipe for XFeedStreamRecipe {
    type Output = XFeedStreamEvent;

    fn hash(&self, state: &mut Hasher) {
        TypeId::of::<Self>().hash(state);
        self.identity.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: EventStream,
    ) -> Pin<Box<dyn futures::Stream<Item = Self::Output> + Send>> {
        x_feed_stream(&self.params)
    }
}

pub(crate) fn x_feed_stream_subscription(
    params: XFeedStreamParams,
) -> Subscription<XFeedStreamEvent> {
    from_recipe(XFeedStreamRecipe {
        identity: XFeedStreamIdentity::for_params(&params),
        params,
    })
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
    let bearer_token = Zeroizing::new(normalize_x_bearer_token_input(&params.bearer_token));
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

    if let Err(err) =
        sync_x_stream_rules(bearer_token.as_str(), &query, params.reconnect_nonce).await
    {
        let _ = send_status(output, false, &err).await;
        return XStreamSessionExit::Retry;
    }

    if !send_status(output, true, "X stream connected").await {
        return XStreamSessionExit::Stop;
    }

    let response = match x_stream_client()
        .get(X_STREAM_ENDPOINT)
        .bearer_auth(bearer_token.as_str())
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
        let preview = sensitive_response_excerpt(&body, 160);
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
                let lines = match extend_x_stream_buffer(&mut buffer, &chunk) {
                    Ok(lines) => lines,
                    Err(err) => {
                        let _ = send_status(output, false, &err).await;
                        return XStreamSessionExit::Retry;
                    }
                };
                for line in lines {
                    let line = line
                        .strip_suffix(b"\n")
                        .unwrap_or(&line)
                        .strip_suffix(b"\r")
                        .unwrap_or(line.as_slice());
                    if line.iter().all(|byte| byte.is_ascii_whitespace()) {
                        continue;
                    }
                    let fetched_at_ms = now_ms();
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

fn extend_x_stream_buffer(buffer: &mut Vec<u8>, chunk: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut lines = Vec::new();
    for segment in chunk.split_inclusive(|byte| *byte == b'\n') {
        if buffer.len().saturating_add(segment.len()) > X_STREAM_MAX_LINE_BYTES {
            buffer.clear();
            return Err(format!(
                "X stream line exceeded {X_STREAM_MAX_LINE_BYTES} bytes before newline"
            ));
        }
        buffer.extend_from_slice(segment);
        if segment.ends_with(b"\n") {
            lines.push(std::mem::take(buffer));
        }
    }
    Ok(lines)
}

pub(crate) async fn clear_x_stream_rules(
    bearer_token: Zeroizing<String>,
    through_generation: u64,
) -> Result<(), String> {
    let bearer_token = Zeroizing::new(normalize_x_bearer_token_input(&bearer_token));
    if bearer_token.is_empty() {
        return Ok(());
    }

    let existing = fetch_x_stream_rules(bearer_token.as_str()).await?;
    let delete_ids = x_stream_rule_cleanup_ids(&existing, through_generation);
    if delete_ids.is_empty() {
        Ok(())
    } else {
        delete_x_stream_rules(bearer_token.as_str(), delete_ids).await
    }
}

async fn sync_x_stream_rules(
    bearer_token: &str,
    query: &str,
    generation: u64,
) -> Result<(), String> {
    let existing = fetch_x_stream_rules(bearer_token).await?;
    let plan = x_stream_rule_sync_plan(&existing, query, generation);
    if plan.delete_ids.is_empty() && plan.add_tag.is_none() {
        return Ok(());
    }

    if !plan.delete_ids.is_empty() {
        delete_x_stream_rules(bearer_token, plan.delete_ids).await?;
    }

    if let Some(tag) = plan.add_tag {
        add_x_stream_rule(bearer_token, query, &tag).await
    } else {
        Ok(())
    }
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

async fn add_x_stream_rule(bearer_token: &str, query: &str, tag: &str) -> Result<(), String> {
    let response = CLIENT
        .post(X_RULES_ENDPOINT)
        .bearer_auth(bearer_token)
        .json(&XRulesAddRequest {
            add: vec![XRuleAdd {
                value: query.to_string(),
                tag: tag.to_string(),
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
    let body = String::from_utf8_lossy(body);
    let raw_preview = body.chars().take(160).collect::<String>();
    let preview = sensitive_response_excerpt(&body, 160);
    if let Some(message) = x_api_auth_guidance(&raw_preview) {
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

#[derive(Debug, PartialEq, Eq)]
struct XRuleSyncPlan {
    delete_ids: Vec<String>,
    add_tag: Option<String>,
}

fn x_stream_rule_sync_plan(existing: &[XRule], query: &str, generation: u64) -> XRuleSyncPlan {
    if existing
        .iter()
        .filter_map(|rule| rule.tag.as_deref())
        .any(|tag| x_rule_tag_is_future_generation(tag, generation))
    {
        return XRuleSyncPlan {
            delete_ids: Vec::new(),
            add_tag: None,
        };
    }

    let current_tag = x_rule_tag_for_generation(generation);
    let delete_ids = x_stream_rule_cleanup_ids(existing, generation);
    let current_rule_matches = existing
        .iter()
        .any(|rule| rule.tag.as_deref() == Some(current_tag.as_str()) && rule.value == query);

    if delete_ids.len() == 1 && current_rule_matches {
        XRuleSyncPlan {
            delete_ids: Vec::new(),
            add_tag: None,
        }
    } else {
        XRuleSyncPlan {
            delete_ids,
            add_tag: Some(current_tag),
        }
    }
}

fn x_stream_rule_cleanup_ids(existing: &[XRule], through_generation: u64) -> Vec<String> {
    existing
        .iter()
        .filter(|rule| {
            rule.tag
                .as_deref()
                .is_some_and(|tag| x_rule_tag_should_cleanup(tag, through_generation))
        })
        .map(|rule| rule.id.clone())
        .collect()
}

fn x_rule_tag_for_generation(generation: u64) -> String {
    format!("{X_RULE_TAG_PREFIX}{}:{generation:x}", x_rule_session_id())
}

fn x_rule_tag_should_cleanup(tag: &str, through_generation: u64) -> bool {
    if tag == X_RULE_TAG {
        return true;
    }

    let Some(suffix) = tag.strip_prefix(X_RULE_TAG_PREFIX) else {
        return false;
    };

    let Some((session_id, generation)) = suffix.split_once(':') else {
        return true;
    };
    if session_id != x_rule_session_id() {
        return true;
    }
    match u64::from_str_radix(generation, 16) {
        Ok(generation) => generation <= through_generation,
        Err(_) => true,
    }
}

fn x_rule_tag_is_future_generation(tag: &str, generation: u64) -> bool {
    tag.strip_prefix(X_RULE_TAG_PREFIX)
        .and_then(|suffix| suffix.split_once(':'))
        .filter(|(session_id, _)| *session_id == x_rule_session_id())
        .and_then(|(_, tag_generation)| u64::from_str_radix(tag_generation, 16).ok())
        .is_some_and(|tag_generation| tag_generation > generation)
}

fn x_rule_session_id() -> &'static str {
    static SESSION_ID: OnceLock<String> = OnceLock::new();
    SESSION_ID.get_or_init(|| format!("{:x}", now_ms()))
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
    fn rule_tag_generation_is_stable_and_compact() {
        let tag = x_rule_tag_for_generation(255);

        assert!(tag.starts_with(X_RULE_TAG_PREFIX));
        assert!(tag.ends_with(":ff"));
        assert!(x_rule_tag_for_generation(u64::MAX).len() <= 48);
    }

    #[test]
    fn stream_rule_cleanup_deletes_legacy_and_current_generations_only() {
        let current_tag = x_rule_tag_for_generation(2);
        let future_tag = x_rule_tag_for_generation(3);
        let rules = vec![
            rule("legacy", "from:old", Some(X_RULE_TAG)),
            rule("current", "from:current", Some(&current_tag)),
            rule("future", "from:future", Some(&future_tag)),
            rule(
                "previous-session",
                "from:old-session",
                Some("kerosene:x-feed:previous:9"),
            ),
            rule("invalid", "from:invalid", Some("kerosene:x-feed:not-hex")),
            rule("foreign", "from:foreign", Some("other-app")),
            rule("untagged", "from:untagged", None),
        ];

        assert_eq!(
            x_stream_rule_cleanup_ids(&rules, 2),
            vec![
                "legacy".to_string(),
                "current".to_string(),
                "previous-session".to_string(),
                "invalid".to_string()
            ]
        );
    }

    #[test]
    fn stream_rule_sync_keeps_matching_current_rule() {
        let current_tag = x_rule_tag_for_generation(2);
        let rules = vec![rule("current", "from:marketfeed", Some(&current_tag))];

        let plan = x_stream_rule_sync_plan(&rules, "from:marketfeed", 2);

        assert_eq!(
            plan,
            XRuleSyncPlan {
                delete_ids: Vec::new(),
                add_tag: None,
            }
        );
    }

    #[test]
    fn stream_rule_sync_replaces_legacy_and_stale_owned_rules() {
        let current_tag = x_rule_tag_for_generation(2);
        let stale_tag = x_rule_tag_for_generation(1);
        let rules = vec![
            rule("legacy", "from:old", Some(X_RULE_TAG)),
            rule("stale", "from:old", Some(&stale_tag)),
            rule("foreign", "from:foreign", Some("other-app")),
        ];

        let plan = x_stream_rule_sync_plan(&rules, "from:marketfeed", 2);

        assert_eq!(
            plan,
            XRuleSyncPlan {
                delete_ids: vec!["legacy".to_string(), "stale".to_string()],
                add_tag: Some(current_tag),
            }
        );
    }

    #[test]
    fn stale_stream_rule_sync_does_not_touch_future_generation() {
        let old_tag = x_rule_tag_for_generation(1);
        let future_tag = x_rule_tag_for_generation(3);
        let rules = vec![
            rule("old", "from:old", Some(&old_tag)),
            rule("future", "from:marketfeed", Some(&future_tag)),
        ];

        let plan = x_stream_rule_sync_plan(&rules, "from:old", 2);

        assert_eq!(
            plan,
            XRuleSyncPlan {
                delete_ids: Vec::new(),
                add_tag: None,
            }
        );
    }

    #[test]
    fn previous_session_rule_does_not_block_current_sync() {
        let current_tag = x_rule_tag_for_generation(0);
        let rules = vec![rule(
            "previous",
            "from:previous",
            Some("kerosene:x-feed:previous:9"),
        )];

        let plan = x_stream_rule_sync_plan(&rules, "from:marketfeed", 0);

        assert_eq!(
            plan,
            XRuleSyncPlan {
                delete_ids: vec!["previous".to_string()],
                add_tag: Some(current_tag),
            }
        );
    }

    #[test]
    fn reconnect_delay_is_capped() {
        assert_eq!(
            next_x_reconnect_delay(Duration::from_secs(40)),
            X_STREAM_RECONNECT_MAX_DELAY
        );
    }

    #[test]
    fn stream_http_client_policy_uses_connect_timeout_without_total_timeout() {
        assert_eq!(
            X_STREAM_HTTP_CLIENT_POLICY.connect_timeout,
            Duration::from_secs(5)
        );
        assert_eq!(X_STREAM_HTTP_CLIENT_POLICY.total_timeout, None);
        assert_eq!(
            X_STREAM_HTTP_CLIENT_POLICY.pool_idle_timeout,
            Duration::from_secs(60)
        );
        assert!(build_x_stream_client(X_STREAM_HTTP_CLIENT_POLICY).is_ok());
    }

    #[test]
    fn stream_params_debug_redacts_bearer_token() {
        let params = XFeedStreamParams {
            bearer_token: Zeroizing::new("secret-bearer-token".to_string()),
            handles: vec!["hyperliquidx".to_string()],
            reconnect_nonce: 7,
        };

        let debug = format!("{params:?}");

        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("hyperliquidx"));
        assert!(!debug.contains("secret-bearer-token"));
    }

    #[test]
    fn stream_identity_excludes_bearer_token_and_tracks_nonce() {
        let left = XFeedStreamParams {
            bearer_token: Zeroizing::new("old-token".to_string()),
            handles: vec!["HyperliquidX".to_string()],
            reconnect_nonce: 1,
        };
        let right = XFeedStreamParams {
            bearer_token: Zeroizing::new("new-token".to_string()),
            handles: vec!["@hyperliquidx".to_string()],
            reconnect_nonce: 1,
        };
        let restarted = XFeedStreamParams {
            bearer_token: Zeroizing::new("new-token".to_string()),
            handles: vec!["@hyperliquidx".to_string()],
            reconnect_nonce: 2,
        };

        assert_eq!(
            XFeedStreamIdentity::for_params(&left),
            XFeedStreamIdentity::for_params(&right)
        );
        assert_ne!(
            XFeedStreamIdentity::for_params(&right),
            XFeedStreamIdentity::for_params(&restarted)
        );
    }

    #[test]
    fn stream_recipe_hash_excludes_bearer_token_and_tracks_nonce() {
        use std::hash::Hasher as _;

        fn recipe_hash(params: XFeedStreamParams) -> u64 {
            let recipe = XFeedStreamRecipe {
                identity: XFeedStreamIdentity::for_params(&params),
                params,
            };
            let mut hasher = Hasher::default();
            recipe.hash(&mut hasher);
            hasher.finish()
        }

        let old_token = XFeedStreamParams {
            bearer_token: Zeroizing::new("old-token".to_string()),
            handles: vec!["HyperliquidX".to_string()],
            reconnect_nonce: 1,
        };
        let new_token_same_nonce = XFeedStreamParams {
            bearer_token: Zeroizing::new("new-token".to_string()),
            handles: vec!["@hyperliquidx".to_string()],
            reconnect_nonce: 1,
        };
        let new_token_restarted = XFeedStreamParams {
            bearer_token: Zeroizing::new("new-token".to_string()),
            handles: vec!["@hyperliquidx".to_string()],
            reconnect_nonce: 2,
        };

        assert_eq!(recipe_hash(old_token), recipe_hash(new_token_same_nonce));
        assert_ne!(
            recipe_hash(XFeedStreamParams {
                bearer_token: Zeroizing::new("new-token".to_string()),
                handles: vec!["@hyperliquidx".to_string()],
                reconnect_nonce: 1,
            }),
            recipe_hash(new_token_restarted)
        );
    }

    #[test]
    fn stream_buffer_drains_complete_lines_and_retains_partial() {
        let mut buffer = Vec::new();

        let lines = extend_x_stream_buffer(&mut buffer, b"{\"a\":1}\r\n{\"b\"").unwrap();
        assert_eq!(lines, vec![b"{\"a\":1}\r\n".to_vec()]);
        assert_eq!(buffer, b"{\"b\"");

        let lines = extend_x_stream_buffer(&mut buffer, b":2}\n").unwrap();
        assert_eq!(lines, vec![b"{\"b\":2}\n".to_vec()]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn stream_buffer_accepts_large_chunks_split_by_newline() {
        let mut buffer = Vec::new();
        let mut chunk = vec![b'a'; X_STREAM_MAX_LINE_BYTES / 2];
        chunk.push(b'\n');
        chunk.extend(std::iter::repeat_n(b'b', X_STREAM_MAX_LINE_BYTES / 2));
        chunk.push(b'\n');

        let lines = extend_x_stream_buffer(&mut buffer, &chunk).unwrap();

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.ends_with(b"\n")));
        assert!(buffer.is_empty());
    }

    #[test]
    fn stream_buffer_rejects_oversized_partial_line_and_clears_buffer() {
        let mut buffer = vec![b'a'; X_STREAM_MAX_LINE_BYTES - 1];

        let err = extend_x_stream_buffer(&mut buffer, b"bb").unwrap_err();

        assert!(err.contains("exceeded"));
        assert!(buffer.is_empty());
    }

    #[test]
    fn stream_buffer_rejects_oversized_complete_line_and_clears_buffer() {
        let mut buffer = Vec::new();
        let mut chunk = vec![b'a'; X_STREAM_MAX_LINE_BYTES];
        chunk.push(b'\n');

        let err = extend_x_stream_buffer(&mut buffer, &chunk).unwrap_err();

        assert!(err.contains("exceeded"));
        assert!(buffer.is_empty());
    }

    fn rule(id: &str, value: &str, tag: Option<&str>) -> XRule {
        XRule {
            id: id.to_string(),
            value: value.to_string(),
            tag: tag.map(str::to_string),
        }
    }
}
