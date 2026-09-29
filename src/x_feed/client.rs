use super::{
    XAuthenticatedUser, XFeedPage, XFeedPost, XFeedRequestError, XFeedSource, XListOwnerKind,
    XListSummary, XListsFetchOutcome, XOAuthTokenRefresh,
};
use crate::api::{CLIENT, KEROSENE_USER_AGENT};
use crate::helpers::redact_sensitive_response_text;
use crate::network_activity::HttpRequestExt as _;
use chrono::{DateTime, Utc};
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use zeroize::Zeroizing;

const X_API_BASE: &str = "https://api.x.com/2";
const X_FEED_REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
// Keep poll payloads small because X API usage is cost-sensitive.
const X_FEED_FETCH_LIMIT: usize = 10;
const X_PROFILE_IMAGE_MAX_BODY_BYTES: usize = 512 * 1024;

pub(crate) async fn fetch_x_auth_context(
    access_token: Zeroizing<String>,
) -> Result<(XAuthenticatedUser, XListsFetchOutcome), String> {
    let user = fetch_x_me(&access_token).await?;
    let lists = fetch_x_lists(access_token, user.id.clone()).await?;
    Ok((user, lists))
}

pub(crate) async fn fetch_x_lists(
    access_token: Zeroizing<String>,
    user_id: String,
) -> Result<XListsFetchOutcome, String> {
    let mut lists = Vec::new();
    let mut unavailable_sources = Vec::new();
    let mut errors = Vec::new();
    let mut successful_sources = 0;

    for owner in [XListOwnerKind::Owned, XListOwnerKind::Followed] {
        match fetch_x_list_page(&access_token, &user_id, owner).await {
            Ok(page) => {
                successful_sources += 1;
                lists.extend(page);
            }
            Err(error) => {
                unavailable_sources.push(owner);
                errors.push(error);
            }
        }
    }

    if successful_sources == 0 {
        return Err(errors.join("; "));
    }

    Ok(XListsFetchOutcome {
        lists: dedup_x_lists(lists),
        unavailable_sources,
    })
}

pub(crate) async fn fetch_x_feed_page(
    access_token: Zeroizing<String>,
    user_id: String,
    source: XFeedSource,
    since_id: Option<String>,
) -> Result<XFeedPage, XFeedRequestError> {
    let url = match &source {
        XFeedSource::Following => {
            format!("{X_API_BASE}/users/{user_id}/timelines/reverse_chronological")
        }
        XFeedSource::List { id, .. } => format!("{X_API_BASE}/lists/{id}/tweets"),
    };
    let mut request = CLIENT
        .get(url)
        .bearer_auth(access_token.as_str())
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .query(&[
            ("max_results", X_FEED_FETCH_LIMIT.to_string()),
            (
                "tweet.fields",
                "author_id,created_at,public_metrics,entities,referenced_tweets".to_string(),
            ),
            ("expansions", "author_id".to_string()),
            (
                "user.fields",
                "username,name,verified,profile_image_url".to_string(),
            ),
        ]);
    if source.supports_since_id()
        && let Some(since_id) = since_id.filter(|id| !id.trim().is_empty())
    {
        request = request.query(&[("since_id", since_id)]);
    }

    let response = request
        .send_observed()
        .await
        .map_err(|e| XFeedRequestError::new(format!("X feed request failed: {e}"), None))?;
    let status = response.status();
    let rate_limited_until_ms = x_response_rate_limited_until_ms(status.as_u16(), &response);
    if !status.is_success() {
        return Err(XFeedRequestError::new(
            x_error_message("X feed request", status.as_u16(), response).await,
            rate_limited_until_ms,
        ));
    }

    let fetched_at_ms = crate::app_time::now_ms();
    let response = response
        .json::<XTimelineResponse>()
        .await
        .map_err(|e| XFeedRequestError::new(format!("X feed response was invalid: {e}"), None))?;
    let mut page = page_from_timeline_response(source, response, fetched_at_ms);
    page.rate_limited_until_ms = rate_limited_until_ms;
    Ok(page)
}

pub(crate) async fn fetch_x_profile_image_bytes(image_url: String) -> Result<Vec<u8>, String> {
    let response = CLIENT
        .get(&image_url)
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .send_observed()
        .await
        .map_err(|e| format!("X profile image request failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("X profile image request failed with HTTP {status}"));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let body = read_x_response_body_limited(response, X_PROFILE_IMAGE_MAX_BODY_BYTES).await?;
    if !is_supported_x_profile_image(&body) {
        let content_type = content_type.unwrap_or_else(|| "unknown content type".to_string());
        return Err(format!(
            "X profile image response was not a supported image: {content_type}"
        ));
    }

    Ok(body)
}

pub(crate) async fn refresh_x_access_token(
    oauth_client_id: Zeroizing<String>,
    refresh_token: Zeroizing<String>,
) -> Result<XOAuthTokenRefresh, String> {
    let response = CLIENT
        .post(format!("{X_API_BASE}/oauth2/token"))
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", oauth_client_id.as_str()),
            ("refresh_token", refresh_token.as_str()),
        ])
        .send_observed()
        .await
        .map_err(|e| format!("X token refresh failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(x_error_message("X token refresh", status.as_u16(), response).await);
    }

    response
        .json::<XOAuthTokenPayload>()
        .await
        .map(|payload| XOAuthTokenRefresh {
            access_token: payload.access_token.into(),
            refresh_token: payload.refresh_token.map(Into::into),
            expires_in_secs: payload.expires_in,
        })
        .map_err(|e| format!("X token refresh response was invalid: {e}"))
}

fn is_supported_x_profile_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"\x89PNG\r\n\x1A\n")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(b"BM")
}

async fn fetch_x_me(access_token: &Zeroizing<String>) -> Result<XAuthenticatedUser, String> {
    let response = CLIENT
        .get(format!("{X_API_BASE}/users/me"))
        .bearer_auth(access_token.as_str())
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .query(&[("user.fields", "username,name,profile_image_url")])
        .send_observed()
        .await
        .map_err(|e| format!("X auth check failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(x_error_message("X auth check", status.as_u16(), response).await);
    }

    response
        .json::<XMeResponse>()
        .await
        .map(|response| XAuthenticatedUser {
            id: response.data.id,
            username: response.data.username,
            name: response.data.name,
        })
        .map_err(|e| format!("X auth response was invalid: {e}"))
}

async fn fetch_x_list_page(
    access_token: &Zeroizing<String>,
    user_id: &str,
    owner: XListOwnerKind,
) -> Result<Vec<XListSummary>, String> {
    let path = match owner {
        XListOwnerKind::Owned => "owned_lists",
        XListOwnerKind::Followed => "followed_lists",
    };
    let response = CLIENT
        .get(format!("{X_API_BASE}/users/{user_id}/{path}"))
        .bearer_auth(access_token.as_str())
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .query(&[("max_results", "100"), ("list.fields", "name,private")])
        .send_observed()
        .await
        .map_err(|e| format!("X list lookup failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(x_error_message("X list lookup", status.as_u16(), response).await);
    }

    let response = response
        .json::<XListsResponse>()
        .await
        .map_err(|e| format!("X list response was invalid: {e}"))?;
    Ok(response
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|list| XListSummary {
            id: list.id,
            name: list.name,
            private: list.private.unwrap_or(false),
            owner,
        })
        .collect())
}

async fn x_error_message(operation: &str, status: u16, response: reqwest::Response) -> String {
    let rate_hint = x_rate_limit_hint(&response);
    let body = response.text().await.unwrap_or_default();
    let body = redact_sensitive_response_text(&body);
    if body.trim().is_empty() {
        format!("{operation} returned HTTP {status}{rate_hint}")
    } else {
        format!("{operation} returned HTTP {status}{rate_hint}: {body}")
    }
}

fn x_rate_limit_hint(response: &reqwest::Response) -> String {
    let remaining = response
        .headers()
        .get("x-rate-limit-remaining")
        .and_then(|value| value.to_str().ok());
    let reset = response
        .headers()
        .get("x-rate-limit-reset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    match (remaining, reset) {
        (Some(remaining), Some(reset)) => format!(
            " (rate remaining {remaining}, reset {})",
            crate::helpers::format_timestamp(reset)
        ),
        (Some(remaining), None) => format!(" (rate remaining {remaining})"),
        _ => String::new(),
    }
}

fn x_response_rate_limited_until_ms(status: u16, response: &reqwest::Response) -> Option<u64> {
    let remaining_is_zero = response
        .headers()
        .get("x-rate-limit-remaining")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "0");
    if status != 429 && !remaining_is_zero {
        return None;
    }

    response
        .headers()
        .get("x-rate-limit-reset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|reset_secs| reset_secs.saturating_mul(1_000))
        .or_else(|| Some(crate::app_time::now_ms().saturating_add(60_000)))
}

fn dedup_x_lists(lists: Vec<XListSummary>) -> Vec<XListSummary> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for list in lists {
        if seen.insert(list.id.clone()) {
            output.push(list);
        }
    }
    output
}

async fn read_x_response_body_limited(
    mut response: reqwest::Response,
    max_body_bytes: usize,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|len| len > max_body_bytes as u64)
    {
        return Err(format!(
            "X profile image response was too large: more than {max_body_bytes} bytes"
        ));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("X profile image response read failed: {e}"))?
    {
        if body.len() + chunk.len() > max_body_bytes {
            return Err(format!(
                "X profile image response was too large: more than {max_body_bytes} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn page_from_timeline_response(
    source: XFeedSource,
    response: XTimelineResponse,
    fetched_at_ms: u64,
) -> XFeedPage {
    let authors = response
        .includes
        .map(|includes| includes.users.unwrap_or_default())
        .unwrap_or_default()
        .into_iter()
        .map(|user| (user.id.clone(), user))
        .collect::<HashMap<_, _>>();

    let posts = response
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|tweet| post_from_tweet(tweet, &authors, fetched_at_ms))
        .collect();

    XFeedPage {
        source,
        posts,
        newest_id: response.meta.and_then(|meta| meta.newest_id),
        rate_limited_until_ms: None,
    }
}

fn post_from_tweet(
    tweet: XTweetPayload,
    authors: &HashMap<String, XUserPayload>,
    fetched_at_ms: u64,
) -> XFeedPost {
    let author = tweet
        .author_id
        .as_ref()
        .and_then(|author_id| authors.get(author_id));
    let author_username = author
        .map(|author| author.username.clone())
        .unwrap_or_else(|| {
            tweet
                .author_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        });
    let author_name = author
        .map(|author| author.name.clone())
        .unwrap_or_else(|| author_username.clone());
    let created_at_ms = tweet
        .created_at
        .as_deref()
        .and_then(parse_x_timestamp_ms)
        .unwrap_or(fetched_at_ms);

    XFeedPost {
        url: format!("https://x.com/{author_username}/status/{}", tweet.id),
        id: tweet.id,
        author_id: tweet.author_id,
        author_name,
        author_username,
        author_profile_image_url: author.and_then(|author| author.profile_image_url.clone()),
        text: tweet.text,
        created_at_ms,
        received_at_ms: fetched_at_ms,
    }
}

fn parse_x_timestamp_ms(value: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
        .and_then(|ms| u64::try_from(ms).ok())
}

#[derive(Debug, Deserialize)]
struct XMeResponse {
    data: XUserPayload,
}

#[derive(Debug, Deserialize)]
struct XListsResponse {
    data: Option<Vec<XListPayload>>,
}

#[derive(Deserialize)]
struct XOAuthTokenPayload {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct XListPayload {
    id: String,
    name: String,
    private: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct XTimelineResponse {
    data: Option<Vec<XTweetPayload>>,
    includes: Option<XTimelineIncludes>,
    meta: Option<XTimelineMeta>,
}

#[derive(Debug, Deserialize)]
struct XTimelineIncludes {
    users: Option<Vec<XUserPayload>>,
}

#[derive(Debug, Deserialize)]
struct XTimelineMeta {
    newest_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XTweetPayload {
    id: String,
    text: String,
    author_id: Option<String>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XUserPayload {
    id: String,
    username: String,
    name: String,
    profile_image_url: Option<String>,
}

#[cfg(test)]
mod tests;
