use crate::api::CLIENT;
use chrono::{DateTime, Utc};
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

const X_API_BASE: &str = "https://api.x.com/2";
const X_WEB_BASE: &str = "https://x.com";
const X_USER_AGENT: &str = "Kerosene X Feed";
const X_FEED_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const X_FEED_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const X_FEED_SEEN_ID_LIMIT: usize = 2048;
pub(crate) const X_FEED_FETCH_LIMIT: usize = 50;
pub(crate) const X_FEED_RENDER_LIMIT: usize = 100;
pub(crate) const X_FEED_MAX_SOURCES: usize = 12;
pub(crate) const X_FEED_REFRESH_INTERVAL_SECS: u64 = 60;
pub(crate) const X_NEW_POST_COOLDOWN_MS: u64 = 120_000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct XTickerMention {
    pub(crate) symbol: String,
    pub(crate) ticker: String,
    pub(crate) reference_price: Option<f64>,
    pub(crate) reference_seen_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct XFeedPost {
    pub(crate) id: String,
    pub(crate) author_id: String,
    pub(crate) username: String,
    pub(crate) text: String,
    pub(crate) timestamp_ms: u64,
    pub(crate) fetched_at_ms: u64,
    pub(crate) request_started_ms: u64,
    pub(crate) request_duration_ms: u64,
    pub(crate) first_seen_ms: u64,
    pub(crate) url: String,
    pub(crate) ticker_mentions: Vec<XTickerMention>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XFeedAuthorProfile {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) name: String,
    pub(crate) initials: String,
    pub(crate) verified: bool,
    pub(crate) avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct XFeedPage {
    pub(crate) profiles: HashMap<String, XFeedAuthorProfile>,
    pub(crate) posts: Vec<XFeedPost>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum XFeedStreamEvent {
    Status { connected: bool, message: String },
    Loaded(Box<Result<XFeedPage, String>>),
}

#[derive(Debug, Clone)]
pub(crate) struct XFeedState {
    pub(crate) handles: Vec<String>,
    pub(crate) notifications_enabled: bool,
    pub(crate) streaming_enabled: bool,
    pub(crate) stream_connected: bool,
    pub(crate) stream_status: Option<(String, bool)>,
    pub(crate) stream_reconnect_nonce: u64,
    pub(crate) bearer_token: Zeroizing<String>,
    pub(crate) bearer_token_input: Zeroizing<String>,
    pub(crate) sources_expanded: bool,
    pub(crate) source_input: String,
    pub(crate) profiles: HashMap<String, XFeedAuthorProfile>,
    pub(crate) posts: Vec<XFeedPost>,
    seen_post_ids: VecDeque<String>,
    pub(crate) loading: bool,
    pub(crate) background_loading: bool,
    pub(crate) last_error: Option<String>,
    pub(crate) last_refresh_ms: Option<u64>,
}

impl XFeedState {
    pub(crate) fn new(
        handles: &[String],
        notifications_enabled: bool,
        streaming_enabled: bool,
        bearer_token: impl Into<String>,
    ) -> Self {
        let bearer_token = Zeroizing::new(bearer_token.into());
        Self {
            handles: normalized_x_handle_list(handles),
            notifications_enabled,
            streaming_enabled,
            stream_connected: false,
            stream_status: None,
            stream_reconnect_nonce: 0,
            bearer_token_input: bearer_token.clone(),
            bearer_token,
            sources_expanded: false,
            source_input: String::new(),
            profiles: HashMap::new(),
            posts: Vec::new(),
            seen_post_ids: VecDeque::new(),
            loading: false,
            background_loading: false,
            last_error: None,
            last_refresh_ms: None,
        }
    }

    pub(crate) fn refreshing(&self) -> bool {
        self.loading || self.background_loading
    }

    pub(crate) fn visible_posts(&self) -> Vec<XFeedPost> {
        let mut posts = self.posts.clone();
        posts.sort_by(|left, right| {
            right
                .timestamp_ms
                .cmp(&left.timestamp_ms)
                .then_with(|| right.id.cmp(&left.id))
        });
        posts.truncate(X_FEED_RENDER_LIMIT);
        posts
    }

    pub(crate) fn has_seen_posts(&self) -> bool {
        !self.seen_post_ids.is_empty() || !self.posts.is_empty()
    }

    pub(crate) fn record_seen_post(&mut self, id: &str) -> bool {
        if self.seen_post_ids.iter().any(|existing| existing == id) {
            return true;
        }

        self.seen_post_ids.push_back(id.to_string());
        while self.seen_post_ids.len() > X_FEED_SEEN_ID_LIMIT {
            let _ = self.seen_post_ids.pop_front();
        }
        false
    }

    pub(crate) fn clear_seen_posts(&mut self) {
        self.seen_post_ids.clear();
    }
}

pub(crate) fn default_x_feed_handles() -> Vec<String> {
    Vec::new()
}

pub(crate) fn normalized_x_handle_list(handles: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for handle in handles {
        if let Ok(handle) = normalize_x_handle_input(handle)
            && !normalized.iter().any(|existing| existing == &handle)
        {
            normalized.push(handle);
            if normalized.len() >= X_FEED_MAX_SOURCES {
                break;
            }
        }
    }
    normalized
}

pub(crate) fn normalize_x_handle_input(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a public X handle".to_string());
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let without_host = without_scheme
        .strip_prefix("x.com/")
        .or_else(|| without_scheme.strip_prefix("www.x.com/"))
        .or_else(|| without_scheme.strip_prefix("twitter.com/"))
        .or_else(|| without_scheme.strip_prefix("www.twitter.com/"))
        .unwrap_or(without_scheme);
    let handle = without_host
        .trim_start_matches('@')
        .split(['?', '#', '/'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    if handle.is_empty() {
        return Err("Enter a public X handle".to_string());
    }
    if !(1..=15).contains(&handle.len()) {
        return Err("X handles must be 1-15 characters".to_string());
    }
    if !handle
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err("X handles can only use letters, numbers, and _".to_string());
    }

    Ok(handle)
}

pub(crate) fn normalize_x_bearer_token_input(input: &str) -> String {
    let mut token = input.trim();
    if let Some(value) = token.strip_prefix("Authorization:") {
        token = value.trim();
    }
    if let Some(value) = token
        .strip_prefix("Bearer ")
        .or_else(|| token.strip_prefix("bearer "))
    {
        token = value.trim();
    }
    token.trim_matches(['"', '\'']).trim().to_string()
}

pub(crate) fn build_x_feed_query(handles: &[String]) -> Result<String, String> {
    let handles = normalized_x_handle_list(handles);
    if handles.is_empty() {
        return Err("Add a public X handle".to_string());
    }

    let sources = handles
        .iter()
        .map(|handle| format!("from:{handle}"))
        .collect::<Vec<_>>();
    let source_query = if sources.len() == 1 {
        sources[0].clone()
    } else {
        format!("({})", sources.join(" OR "))
    };

    Ok(format!("{source_query} -is:retweet"))
}

pub(crate) async fn fetch_x_recent_posts(
    bearer_token: String,
    handles: Vec<String>,
) -> Result<XFeedPage, String> {
    let bearer_token = normalize_x_bearer_token_input(&bearer_token);
    if bearer_token.is_empty() {
        return Err("Enter an X API bearer token".to_string());
    }
    let handles = normalized_x_handle_list(&handles);
    let query = build_x_feed_query(&handles)?;
    let url = format!("{X_API_BASE}/tweets/search/recent");
    let request_started_ms = system_time_ms();
    let max_results = X_FEED_FETCH_LIMIT.to_string();
    let response = CLIENT
        .get(&url)
        .bearer_auth(&bearer_token)
        .header(USER_AGENT, X_USER_AGENT)
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .query(&[
            ("query", query.as_str()),
            ("max_results", max_results.as_str()),
            (
                "tweet.fields",
                "created_at,author_id,entities,public_metrics",
            ),
            ("expansions", "author_id"),
            ("user.fields", "username,name,profile_image_url,verified"),
        ])
        .send()
        .await
        .map_err(|e| format!("X recent search request failed: {e}"))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("X recent search response read failed: {e}"))?;
    let fetched_at_ms = system_time_ms();
    let request_duration_ms = fetched_at_ms.saturating_sub(request_started_ms);

    if body.len() > X_FEED_MAX_BODY_BYTES {
        return Err(format!(
            "X recent search response was too large: {} bytes",
            body.len()
        ));
    }
    if !status.is_success() {
        let preview = String::from_utf8_lossy(&body)
            .chars()
            .take(160)
            .collect::<String>();
        return if let Some(message) = x_api_auth_guidance(&preview) {
            Err(message)
        } else if preview.is_empty() {
            Err(format!("X recent search failed with HTTP {status}"))
        } else {
            Err(format!(
                "X recent search failed with HTTP {status}: {preview}"
            ))
        };
    }

    let response: XRecentSearchResponse = serde_json::from_slice(&body).map_err(|e| {
        let content_type = content_type.unwrap_or_else(|| "unknown content type".to_string());
        format!("X recent search response parse failed ({content_type}): {e}")
    })?;

    let page = x_feed_page_from_parts(
        response.data.unwrap_or_default(),
        response.includes.unwrap_or_default(),
        request_started_ms,
        fetched_at_ms,
        request_duration_ms,
    );
    Ok(page)
}

pub(crate) fn parse_x_stream_page(body: &[u8], fetched_at_ms: u64) -> Result<XFeedPage, String> {
    let response: XStreamResponse =
        serde_json::from_slice(body).map_err(|e| format!("X stream response parse failed: {e}"))?;
    if let Some(errors) = response.errors
        && !errors.is_empty()
        && response.data.is_none()
    {
        return Err(format!(
            "X stream error: {}",
            errors
                .into_iter()
                .filter_map(|err| err.title.or(err.detail))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    Ok(x_feed_page_from_parts(
        response.data.into_iter().collect(),
        response.includes.unwrap_or_default(),
        fetched_at_ms,
        fetched_at_ms,
        0,
    ))
}

pub(crate) fn x_age_countdown_label(sent_at_ms: u64, now_ms: u64) -> String {
    format!(
        "{} ago",
        x_countdown_duration_label(now_ms.saturating_sub(sent_at_ms))
    )
}

pub(crate) fn x_new_post_heat(first_seen_ms: u64, now_ms: u64) -> f32 {
    if first_seen_ms == 0 {
        return 0.0;
    }

    let age_ms = now_ms.saturating_sub(first_seen_ms);
    if age_ms >= X_NEW_POST_COOLDOWN_MS {
        0.0
    } else {
        1.0 - (age_ms as f32 / X_NEW_POST_COOLDOWN_MS as f32)
    }
}

pub(crate) fn x_arrival_latency_label(post: &XFeedPost) -> Option<String> {
    if post.fetched_at_ms == 0 || post.first_seen_ms == 0 {
        return None;
    }

    Some(format!(
        "seen +{}",
        x_duration_label(post.fetched_at_ms.saturating_sub(post.timestamp_ms))
    ))
}

pub(crate) fn x_price_impact_pct(
    reference_price: Option<f64>,
    current_price: Option<f64>,
) -> Option<f64> {
    let reference = reference_price.filter(|price| price.is_finite() && *price > 0.0)?;
    let current = current_price.filter(|price| price.is_finite() && *price > 0.0)?;
    Some(((current / reference) - 1.0) * 100.0)
}

pub(crate) fn x_api_auth_guidance(body: &str) -> Option<String> {
    if body.contains("client-not-enrolled") || body.contains("attached to a Project") {
        Some(
            "X rejected this token. Use the Bearer Token from an X developer App attached to a Project: Developer Portal -> Project -> App -> Keys and tokens. Standalone app credentials will not work for X API v2."
                .to_string(),
        )
    } else {
        None
    }
}

fn x_feed_page_from_parts(
    posts: Vec<XPostPayload>,
    includes: XIncludesPayload,
    request_started_ms: u64,
    fetched_at_ms: u64,
    request_duration_ms: u64,
) -> XFeedPage {
    let profiles = includes
        .users
        .into_iter()
        .map(|user| {
            let profile = x_profile_from_payload(user);
            (profile.id.clone(), profile)
        })
        .collect::<HashMap<_, _>>();

    let mut posts = posts
        .into_iter()
        .filter_map(|post| x_post_from_payload(post, &profiles))
        .map(|mut post| {
            post.request_started_ms = request_started_ms;
            post.fetched_at_ms = fetched_at_ms;
            post.request_duration_ms = request_duration_ms;
            post
        })
        .collect::<Vec<_>>();
    posts.sort_by(|left, right| {
        right
            .timestamp_ms
            .cmp(&left.timestamp_ms)
            .then_with(|| right.id.cmp(&left.id))
    });

    XFeedPage { profiles, posts }
}

fn x_profile_from_payload(user: XUserPayload) -> XFeedAuthorProfile {
    let username = user.username.to_ascii_lowercase();
    let name = user.name.unwrap_or_else(|| format!("@{username}"));
    XFeedAuthorProfile {
        id: user.id,
        initials: profile_initials(&name, &username),
        username,
        name,
        verified: user.verified.unwrap_or(false),
        avatar_url: user.profile_image_url,
    }
}

fn x_post_from_payload(
    post: XPostPayload,
    profiles: &HashMap<String, XFeedAuthorProfile>,
) -> Option<XFeedPost> {
    let author_id = post.author_id?;
    let profile = profiles.get(&author_id)?;
    let timestamp_ms = post
        .created_at
        .as_deref()
        .and_then(parse_x_timestamp_ms)
        .unwrap_or_else(system_time_ms);
    let text = normalize_x_text(&post.text);
    if text.trim().is_empty() {
        return None;
    }

    Some(XFeedPost {
        url: format!("{X_WEB_BASE}/{}/status/{}", profile.username, post.id),
        id: post.id,
        author_id,
        username: profile.username.clone(),
        text,
        timestamp_ms,
        fetched_at_ms: 0,
        request_started_ms: 0,
        request_duration_ms: 0,
        first_seen_ms: 0,
        ticker_mentions: Vec::new(),
    })
}

fn normalize_x_text(input: &str) -> String {
    input
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_x_timestamp_ms(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()?
        .with_timezone(&Utc)
        .timestamp_millis()
        .try_into()
        .ok()
}

fn profile_initials(name: &str, username: &str) -> String {
    let mut initials = name
        .split_whitespace()
        .filter_map(|part| part.chars().find(|ch| ch.is_ascii_alphanumeric()))
        .take(2)
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>();
    if initials.is_empty() {
        initials = username
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .take(2)
            .map(|ch| ch.to_ascii_uppercase())
            .collect();
    }
    if initials.is_empty() {
        "?".to_string()
    } else {
        initials
    }
}

fn system_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn x_countdown_duration_label(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        format!("{duration_ms} ms")
    } else if duration_ms < 60_000 {
        format!("{}s", duration_ms / 1_000)
    } else if duration_ms < 3_600_000 {
        format!("{}m", duration_ms / 60_000)
    } else if duration_ms < 86_400_000 {
        format!("{}h", duration_ms / 3_600_000)
    } else {
        format!("{}d", duration_ms / 86_400_000)
    }
}

fn x_duration_label(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        format!("{duration_ms} ms")
    } else if duration_ms < 60_000 {
        format!("{}.{:03} s", duration_ms / 1_000, duration_ms % 1_000)
    } else if duration_ms < 3_600_000 {
        format!(
            "{}m {}s",
            duration_ms / 60_000,
            (duration_ms % 60_000) / 1_000
        )
    } else {
        format!(
            "{}h {}m",
            duration_ms / 3_600_000,
            (duration_ms % 3_600_000) / 60_000
        )
    }
}

#[derive(Debug, Deserialize)]
struct XRecentSearchResponse {
    data: Option<Vec<XPostPayload>>,
    includes: Option<XIncludesPayload>,
}

#[derive(Debug, Deserialize)]
struct XStreamResponse {
    data: Option<XPostPayload>,
    includes: Option<XIncludesPayload>,
    errors: Option<Vec<XErrorPayload>>,
}

#[derive(Debug, Deserialize)]
struct XErrorPayload {
    title: Option<String>,
    detail: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct XIncludesPayload {
    #[serde(default)]
    users: Vec<XUserPayload>,
}

#[derive(Debug, Deserialize)]
struct XUserPayload {
    id: String,
    username: String,
    name: Option<String>,
    profile_image_url: Option<String>,
    verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct XPostPayload {
    id: String,
    text: String,
    author_id: Option<String>,
    created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_handle_normalization_accepts_common_forms() {
        assert_eq!(
            normalize_x_handle_input("@HyperliquidX").unwrap(),
            "hyperliquidx"
        );
        assert_eq!(
            normalize_x_handle_input("https://x.com/HyperliquidX/status/1").unwrap(),
            "hyperliquidx"
        );
        assert_eq!(
            normalize_x_handle_input("twitter.com/HyperliquidX?lang=en").unwrap(),
            "hyperliquidx"
        );
    }

    #[test]
    fn x_handle_normalization_rejects_invalid_handles() {
        assert!(normalize_x_handle_input("").is_err());
        assert!(normalize_x_handle_input("bad-handle").is_err());
        assert!(normalize_x_handle_input("thishandleistoolong").is_err());
    }

    #[test]
    fn x_feed_query_combines_sources_and_filters_retweets() {
        let query = build_x_feed_query(&["foo".to_string(), "@bar".to_string()]).unwrap();
        assert_eq!(query, "(from:foo OR from:bar) -is:retweet");
    }

    #[test]
    fn x_bearer_token_normalization_accepts_copied_header_values() {
        assert_eq!(normalize_x_bearer_token_input(" token "), "token");
        assert_eq!(normalize_x_bearer_token_input("Bearer token"), "token");
        assert_eq!(
            normalize_x_bearer_token_input("Authorization: Bearer \"token\""),
            "token"
        );
    }

    #[test]
    fn x_api_auth_guidance_detects_project_enrollment_errors() {
        let body = r#"{"reason":"client-not-enrolled","detail":"When authenticating requests to the Twitter API v2 endpoints, you must use keys and tokens from a Twitter developer App that is attached to a Project."}"#;

        assert!(
            x_api_auth_guidance(body)
                .unwrap()
                .contains("attached to a Project")
        );
    }

    #[test]
    fn x_recent_response_parses_posts_and_profiles() {
        let json = br#"{
            "data": [{
                "id": "181",
                "author_id": "7",
                "text": "BTC moves",
                "created_at": "2026-06-01T10:00:00.000Z"
            }],
            "includes": {
                "users": [{
                    "id": "7",
                    "username": "MarketFeed",
                    "name": "Market Feed",
                    "verified": true,
                    "profile_image_url": "https://example.com/avatar.jpg"
                }]
            }
        }"#;

        let response: XRecentSearchResponse = serde_json::from_slice(json).unwrap();
        let page =
            x_feed_page_from_parts(response.data.unwrap(), response.includes.unwrap(), 1, 2, 1);

        assert_eq!(page.posts.len(), 1);
        assert_eq!(page.posts[0].username, "marketfeed");
        assert_eq!(page.posts[0].url, "https://x.com/marketfeed/status/181");
        assert_eq!(page.profiles["7"].name, "Market Feed");
    }
}
