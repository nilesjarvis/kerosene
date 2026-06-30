use crate::api::{CLIENT, KEROSENE_USER_AGENT};
use crate::app_state::{SensitiveString, sensitive_string};
use crate::helpers::{fallback_initials, redact_sensitive_response_text};
use chrono::{DateTime, Utc};
use iced::widget::image::Handle as ImageHandle;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;
use zeroize::{Zeroize, Zeroizing};

const X_API_BASE: &str = "https://api.x.com/2";
const X_FEED_REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
pub(crate) const X_FEED_REFRESH_INTERVAL_SECS: u64 = 10;
pub(crate) const X_FEED_POST_LIMIT: usize = 100;
const X_FEED_FETCH_LIMIT: usize = 50;
const X_PROFILE_IMAGE_MAX_BODY_BYTES: usize = 512 * 1024;
pub(crate) const X_PROFILE_IMAGE_RETRY_BACKOFF_MS: u64 = 300_000;

pub(crate) type XFeedId = u64;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum XFeedSource {
    #[default]
    Following,
    List {
        id: String,
        name: String,
        #[serde(default)]
        private: bool,
    },
}

impl fmt::Debug for XFeedSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Following => f.write_str("Following"),
            Self::List { .. } => f
                .debug_struct("List")
                .field("id", &"<redacted>")
                .field("name", &"<redacted>")
                .finish(),
        }
    }
}

impl XFeedSource {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Following => "Following".to_string(),
            Self::List { name, .. } if !name.trim().is_empty() => format!("List · {name}"),
            Self::List { id, .. } => format!("List · {id}"),
        }
    }

    pub(crate) fn key(&self) -> String {
        match self {
            Self::Following => "home".to_string(),
            Self::List { id, .. } => format!("list:{id}"),
        }
    }

    pub(crate) fn supports_since_id(&self) -> bool {
        matches!(self, Self::Following)
    }

    pub(crate) fn is_private(&self) -> bool {
        matches!(self, Self::List { private: true, .. })
    }

    fn debug_label(&self) -> &'static str {
        match self {
            Self::Following => "home",
            Self::List { .. } => "list:<redacted>",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XFeedSourceOption {
    pub(crate) source: XFeedSource,
    label: String,
}

impl fmt::Debug for XFeedSourceOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedSourceOption")
            .field("source", &self.source)
            .field("label", &"<redacted>")
            .finish()
    }
}

impl XFeedSourceOption {
    pub(crate) fn new(source: XFeedSource) -> Self {
        let label = source.label();
        Self { source, label }
    }
}

impl fmt::Display for XFeedSourceOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XAuthenticatedUser {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) name: String,
}

impl fmt::Debug for XAuthenticatedUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XAuthenticatedUser")
            .field("id", &"<redacted>")
            .field("username", &"<redacted>")
            .field("name", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct XListSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) private: bool,
    pub(crate) owner: XListOwnerKind,
}

impl fmt::Debug for XListSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XListSummary")
            .field("id", &"<redacted>")
            .field("name", &"<redacted>")
            .field("owner", &self.owner)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum XListOwnerKind {
    Owned,
    Followed,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XListsFetchOutcome {
    pub(crate) lists: Vec<XListSummary>,
    pub(crate) unavailable_sources: Vec<XListOwnerKind>,
}

impl fmt::Debug for XListsFetchOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XListsFetchOutcome")
            .field("lists", &self.lists.len())
            .field("unavailable_sources", &self.unavailable_sources)
            .finish()
    }
}

impl XListsFetchOutcome {
    pub(crate) fn status_suffix(&self) -> String {
        match self.unavailable_sources.len() {
            0 => String::new(),
            1 => format!(
                "; {} List source unavailable",
                self.unavailable_sources[0].label()
            ),
            count => format!("; {count} List sources unavailable"),
        }
    }
}

impl XListOwnerKind {
    fn label(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Followed => "followed",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XFeedPost {
    pub(crate) id: String,
    pub(crate) author_id: Option<String>,
    pub(crate) author_name: String,
    pub(crate) author_username: String,
    pub(crate) author_profile_image_url: Option<String>,
    pub(crate) text: String,
    pub(crate) created_at_ms: u64,
    pub(crate) received_at_ms: u64,
    pub(crate) url: String,
}

impl fmt::Debug for XFeedPost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedPost")
            .field("id", &self.id)
            .field("author_id", &self.author_id.as_ref().map(|_| "<redacted>"))
            .field("author_name", &"<redacted>")
            .field("author_username", &"<redacted>")
            .field(
                "author_profile_image_url",
                &self.author_profile_image_url.as_ref().map(|_| "<url>"),
            )
            .field("text", &"<redacted>")
            .field("created_at_ms", &self.created_at_ms)
            .field("received_at_ms", &self.received_at_ms)
            .field("url", &"<redacted>")
            .finish()
    }
}

impl XFeedPost {
    pub(crate) fn author_profile_key(&self) -> String {
        x_author_profile_key(self.author_id.as_deref(), &self.author_username)
    }

    pub(crate) fn author_initials(&self) -> String {
        fallback_initials(&self.author_name, &self.author_username)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XFeedPage {
    pub(crate) source: XFeedSource,
    pub(crate) posts: Vec<XFeedPost>,
    pub(crate) newest_id: Option<String>,
    pub(crate) rate_limited_until_ms: Option<u64>,
}

impl fmt::Debug for XFeedPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedPage")
            .field("source", &self.source.debug_label())
            .field("posts", &self.posts.len())
            .field("newest_id", &self.newest_id)
            .field("rate_limited_until_ms", &self.rate_limited_until_ms)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XFeedRequestError {
    pub(crate) message: String,
    pub(crate) rate_limited_until_ms: Option<u64>,
}

impl XFeedRequestError {
    pub(crate) fn new(message: String, rate_limited_until_ms: Option<u64>) -> Self {
        Self {
            message,
            rate_limited_until_ms,
        }
    }

    #[cfg(test)]
    pub(crate) fn plain(message: impl Into<String>) -> Self {
        Self::new(message.into(), None)
    }
}

impl fmt::Debug for XFeedRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedRequestError")
            .field("message", &redact_sensitive_response_text(&self.message))
            .field("rate_limited_until_ms", &self.rate_limited_until_ms)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct XFeedInstance {
    pub(crate) id: XFeedId,
    pub(crate) source: XFeedSource,
    pub(crate) posts: Vec<XFeedPost>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_refresh_ms: Option<u64>,
}

impl fmt::Debug for XFeedInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedInstance")
            .field("id", &self.id)
            .field("source", &self.source)
            .field("posts", &self.posts.len())
            .field(
                "last_error",
                &self
                    .last_error
                    .as_ref()
                    .map(|error| redact_sensitive_response_text(error)),
            )
            .field("last_refresh_ms", &self.last_refresh_ms)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct XAuthorProfile {
    pub(crate) author_id: Option<String>,
    pub(crate) username: String,
    pub(crate) name: String,
    pub(crate) initials: String,
    pub(crate) profile_image_url: Option<String>,
    pub(crate) image_handle: Option<ImageHandle>,
    pub(crate) image_loading_url: Option<String>,
    pub(crate) image_request_id: u64,
    pub(crate) image_failed_at_ms: Option<u64>,
}

impl fmt::Debug for XAuthorProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XAuthorProfile")
            .field("author_id", &self.author_id.as_ref().map(|_| "<redacted>"))
            .field("username", &"<redacted>")
            .field("name", &"<redacted>")
            .field("initials", &"<redacted>")
            .field(
                "profile_image_url",
                &self.profile_image_url.as_ref().map(|_| "<url>"),
            )
            .field(
                "image_handle",
                &self.image_handle.as_ref().map(|_| "<image>"),
            )
            .field(
                "image_loading_url",
                &self.image_loading_url.as_ref().map(|_| "<url>"),
            )
            .field("image_request_id", &self.image_request_id)
            .field("image_failed_at_ms", &self.image_failed_at_ms)
            .finish()
    }
}

impl XAuthorProfile {
    pub(crate) fn from_post(post: &XFeedPost) -> Self {
        Self {
            author_id: post.author_id.clone(),
            username: post.author_username.clone(),
            name: post.author_name.clone(),
            initials: post.author_initials(),
            profile_image_url: post.author_profile_image_url.clone(),
            image_handle: None,
            image_loading_url: None,
            image_request_id: 0,
            image_failed_at_ms: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct XFeedState {
    pub(crate) access_token_input: SensitiveString,
    pending_access_token: SensitiveString,
    access_token: SensitiveString,
    pub(crate) auth_user: Option<XAuthenticatedUser>,
    pub(crate) lists: Vec<XListSummary>,
    pub(crate) connect_request_id: u64,
    pub(crate) lists_request_id: u64,
    pub(crate) refresh_request_id: u64,
    source_refresh_request_ids: HashMap<String, u64>,
    source_rate_limit_reset_ms: HashMap<String, u64>,
    pub(crate) connecting: bool,
    pub(crate) lists_loading: bool,
    pub(crate) status: Option<(String, bool)>,
    pub(crate) instances: HashMap<XFeedId, XFeedInstance>,
    pub(crate) author_profiles: HashMap<String, XAuthorProfile>,
    pub(crate) next_profile_image_request_id: u64,
}

impl fmt::Debug for XFeedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedState")
            .field("access_token_input", &"<redacted>")
            .field("pending_access_token", &"<redacted>")
            .field("access_token", &"<redacted>")
            .field("auth_user", &self.auth_user)
            .field("lists", &self.lists.len())
            .field("connect_request_id", &self.connect_request_id)
            .field("lists_request_id", &self.lists_request_id)
            .field("refresh_request_id", &self.refresh_request_id)
            .field(
                "source_refresh_request_ids",
                &self.source_refresh_request_ids.len(),
            )
            .field(
                "source_rate_limit_reset_ms",
                &self.source_rate_limit_reset_ms.len(),
            )
            .field("connecting", &self.connecting)
            .field("lists_loading", &self.lists_loading)
            .field(
                "status",
                &self
                    .status
                    .as_ref()
                    .map(|(message, is_error)| (redact_sensitive_response_text(message), is_error)),
            )
            .field("instances", &self.instances.len())
            .field("author_profiles", &self.author_profiles.len())
            .field(
                "next_profile_image_request_id",
                &self.next_profile_image_request_id,
            )
            .finish()
    }
}

impl XFeedState {
    pub(crate) fn new(configs: &[crate::config::XFeedConfig], access_token: &str) -> Self {
        let mut instances = HashMap::new();
        for config in configs {
            instances.insert(
                config.id,
                XFeedInstance::new(config.id, config.source.clone()),
            );
        }
        let access_token = access_token.trim().to_string();

        Self {
            access_token_input: sensitive_string(String::new()),
            pending_access_token: sensitive_string(String::new()),
            access_token: sensitive_string(access_token),
            auth_user: None,
            lists: Vec::new(),
            connect_request_id: 0,
            lists_request_id: 0,
            refresh_request_id: 0,
            source_refresh_request_ids: HashMap::new(),
            source_rate_limit_reset_ms: HashMap::new(),
            connecting: false,
            lists_loading: false,
            status: None,
            instances,
            author_profiles: HashMap::new(),
            next_profile_image_request_id: 0,
        }
    }

    pub(crate) fn has_access_token(&self) -> bool {
        !self.access_token.trim().is_empty()
    }

    pub(crate) fn loading(&self) -> bool {
        self.connecting || self.lists_loading || !self.source_refresh_request_ids.is_empty()
    }

    pub(crate) fn access_token_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.access_token.trim().to_string())
    }

    pub(crate) fn access_token_for_secret(&self) -> Zeroizing<String> {
        Zeroizing::new(self.access_token.trim().to_string())
    }

    pub(crate) fn access_token_candidate_from_input(&mut self) -> Option<Zeroizing<String>> {
        let token = self.access_token_input.trim().to_string();
        if token.is_empty() {
            self.status = Some(("Paste an X OAuth 2.0 user access token".to_string(), true));
            return None;
        }

        self.pending_access_token.zeroize();
        self.pending_access_token = sensitive_string(token.clone());
        self.access_token_input.zeroize();
        Some(Zeroizing::new(token))
    }

    pub(crate) fn commit_access_token(&mut self, token: &str) -> bool {
        let changed = self.set_access_token_from_secret(token);
        self.access_token_input.zeroize();
        self.pending_access_token.zeroize();
        changed
    }

    pub(crate) fn pending_access_token_for_secret(&self) -> Option<Zeroizing<String>> {
        let token = self.pending_access_token.trim().to_string();
        (!token.is_empty()).then(|| Zeroizing::new(token))
    }

    pub(crate) fn clear_pending_access_token(&mut self) {
        self.pending_access_token.zeroize();
    }

    pub(crate) fn set_access_token_from_secret(&mut self, token: &str) -> bool {
        let token = token.trim();
        let changed = self.access_token.trim() != token;
        if changed {
            self.invalidate_requests();
            self.auth_user = None;
            self.lists.clear();
            self.author_profiles.clear();
            self.connecting = false;
            self.lists_loading = false;
            for instance in self.instances.values_mut() {
                if token.is_empty() || instance.source.is_private() {
                    instance.source = XFeedSource::Following;
                }
                instance.last_error = None;
                instance.posts.clear();
                instance.last_refresh_ms = None;
            }
        }

        self.access_token.zeroize();
        self.access_token = sensitive_string(token.to_string());
        changed
    }

    pub(crate) fn clear_access_token(&mut self) {
        self.access_token_input.zeroize();
        self.pending_access_token.zeroize();
        self.set_access_token_from_secret("");
        self.status = Some(("X token cleared".to_string(), false));
    }

    pub(crate) fn invalidate_requests(&mut self) {
        self.connect_request_id = self.connect_request_id.saturating_add(1);
        self.lists_request_id = self.lists_request_id.saturating_add(1);
        self.refresh_request_id = self.refresh_request_id.saturating_add(1);
        self.source_refresh_request_ids.clear();
        self.source_rate_limit_reset_ms.clear();
    }

    pub(crate) fn next_connect_request_id(&mut self) -> u64 {
        self.connect_request_id = self.connect_request_id.saturating_add(1);
        self.connect_request_id
    }

    pub(crate) fn next_lists_request_id(&mut self) -> u64 {
        self.lists_request_id = self.lists_request_id.saturating_add(1);
        self.lists_request_id
    }

    pub(crate) fn begin_source_refresh(&mut self, source: &XFeedSource) -> u64 {
        self.refresh_request_id = self.refresh_request_id.saturating_add(1);
        let request_id = self.refresh_request_id;
        self.source_refresh_request_ids
            .insert(source.key(), request_id);
        request_id
    }

    pub(crate) fn finish_source_refresh(&mut self, source: &XFeedSource, request_id: u64) -> bool {
        let key = source.key();
        if self
            .source_refresh_request_ids
            .get(&key)
            .is_some_and(|current_id| *current_id == request_id)
        {
            self.source_refresh_request_ids.remove(&key);
            true
        } else {
            false
        }
    }

    pub(crate) fn source_refresh_in_flight(&self, source: &XFeedSource) -> bool {
        self.source_refresh_request_ids.contains_key(&source.key())
    }

    pub(crate) fn source_rate_limited_until(
        &mut self,
        source: &XFeedSource,
        now_ms: u64,
    ) -> Option<u64> {
        let key = source.key();
        match self.source_rate_limit_reset_ms.get(&key).copied() {
            Some(reset_ms) if reset_ms > now_ms => Some(reset_ms),
            Some(_) => {
                self.source_rate_limit_reset_ms.remove(&key);
                None
            }
            None => None,
        }
    }

    pub(crate) fn set_source_rate_limit(&mut self, source: &XFeedSource, reset_ms: u64) {
        self.source_rate_limit_reset_ms
            .insert(source.key(), reset_ms);
    }

    pub(crate) fn persistable_source(&self, source: &XFeedSource) -> XFeedSource {
        if source.is_private() {
            XFeedSource::Following
        } else {
            source.clone()
        }
    }

    pub(crate) fn source_options(&self) -> Vec<XFeedSourceOption> {
        let mut options = vec![XFeedSourceOption::new(XFeedSource::Following)];
        let mut seen_lists = HashSet::new();
        let mut lists = self.lists.clone();
        lists.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });
        for list in lists {
            if seen_lists.insert(list.id.clone()) {
                options.push(XFeedSourceOption::new(XFeedSource::List {
                    id: list.id,
                    name: list.name,
                    private: list.private,
                }));
            }
        }
        options
    }

    pub(crate) fn author_profile_for_post(&self, post: &XFeedPost) -> Option<&XAuthorProfile> {
        self.author_profiles.get(&post.author_profile_key())
    }
}

impl XFeedInstance {
    pub(crate) fn new(id: XFeedId, source: XFeedSource) -> Self {
        Self {
            id,
            source,
            posts: Vec::new(),
            last_error: None,
            last_refresh_ms: None,
        }
    }

    pub(crate) fn apply_page(&mut self, page: &XFeedPage, now_ms: u64) {
        let mut seen = self
            .posts
            .iter()
            .map(|post| post.id.clone())
            .collect::<HashSet<_>>();

        for post in &page.posts {
            if seen.insert(post.id.clone()) {
                self.posts.push(post.clone());
            }
        }

        self.posts.sort_by(|a, b| {
            b.created_at_ms
                .cmp(&a.created_at_ms)
                .then_with(|| b.id.cmp(&a.id))
        });
        if self.posts.len() > X_FEED_POST_LIMIT {
            self.posts.truncate(X_FEED_POST_LIMIT);
        }
        self.last_refresh_ms = Some(now_ms);
        self.last_error = None;
    }

    pub(crate) fn newest_seen_id(&self) -> Option<String> {
        self.posts
            .iter()
            .filter_map(|post| post.id.parse::<u64>().ok().map(|id| (id, post.id.clone())))
            .max_by_key(|(id, _)| *id)
            .map(|(_, id)| id)
    }
}

pub(crate) async fn fetch_x_auth_context(
    access_token: Zeroizing<String>,
) -> Result<(XAuthenticatedUser, XListsFetchOutcome), String> {
    let user = fetch_x_me(access_token.clone()).await?;
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
        .send()
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
        .send()
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

fn is_supported_x_profile_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"\x89PNG\r\n\x1A\n")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(b"BM")
}

async fn fetch_x_me(access_token: Zeroizing<String>) -> Result<XAuthenticatedUser, String> {
    let response = CLIENT
        .get(format!("{X_API_BASE}/users/me"))
        .bearer_auth(access_token.as_str())
        .timeout(X_FEED_REQUEST_TIMEOUT)
        .header(USER_AGENT, KEROSENE_USER_AGENT)
        .query(&[("user.fields", "username,name,profile_image_url")])
        .send()
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
        .send()
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

fn x_author_profile_key(author_id: Option<&str>, username: &str) -> String {
    author_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(|id| format!("id:{id}"))
        .unwrap_or_else(|| format!("username:{}", username.to_ascii_lowercase()))
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
mod tests {
    use super::*;

    #[test]
    fn x_feed_state_debug_redacts_tokens_and_status() {
        let mut state = XFeedState::new(&[], "");
        state.access_token_input = sensitive_string("token-input");
        state.access_token = sensitive_string("saved-token");
        state.status = Some(("token-input failed".to_string(), true));

        let rendered = format!("{state:?}");

        assert!(!rendered.contains("token-input"));
        assert!(!rendered.contains("saved-token"));
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn x_feed_instance_dedupes_and_sorts_posts() {
        let mut instance = XFeedInstance::new(0, XFeedSource::Following);
        let page = XFeedPage {
            source: XFeedSource::Following,
            posts: vec![
                test_post("1", 1_000),
                test_post("2", 2_000),
                test_post("1", 1_000),
            ],
            newest_id: Some("2".to_string()),
            rate_limited_until_ms: None,
        };

        instance.apply_page(&page, 3_000);

        assert_eq!(instance.posts.len(), 2);
        assert_eq!(instance.posts[0].id, "2");
        assert_eq!(instance.newest_seen_id().as_deref(), Some("2"));
    }

    #[test]
    fn x_feed_source_options_dedupe_lists() {
        let mut state = XFeedState::new(&[], "");
        state.lists = vec![
            XListSummary {
                id: "10".to_string(),
                name: "Macro".to_string(),
                private: false,
                owner: XListOwnerKind::Owned,
            },
            XListSummary {
                id: "10".to_string(),
                name: "Macro copy".to_string(),
                private: false,
                owner: XListOwnerKind::Followed,
            },
        ];

        let options = state.source_options();

        assert_eq!(options.len(), 2);
        assert!(matches!(options[0].source, XFeedSource::Following));
        assert_eq!(options[1].source.key(), "list:10");
    }

    #[test]
    fn timeline_response_carries_author_profile_image_urls() {
        let page = page_from_timeline_response(
            XFeedSource::Following,
            XTimelineResponse {
                data: Some(vec![XTweetPayload {
                    id: "99".to_string(),
                    text: "hello".to_string(),
                    author_id: Some("42".to_string()),
                    created_at: Some("2026-06-30T12:00:00.000Z".to_string()),
                }]),
                includes: Some(XTimelineIncludes {
                    users: Some(vec![XUserPayload {
                        id: "42".to_string(),
                        username: "alice".to_string(),
                        name: "Alice".to_string(),
                        profile_image_url: Some("https://example.com/alice.jpg".to_string()),
                    }]),
                }),
                meta: None,
            },
            1_000,
        );

        assert_eq!(
            page.posts[0].author_profile_image_url.as_deref(),
            Some("https://example.com/alice.jpg")
        );
        assert_eq!(page.posts[0].author_profile_key(), "id:42");
    }

    fn test_post(id: &str, created_at_ms: u64) -> XFeedPost {
        XFeedPost {
            id: id.to_string(),
            author_id: Some("42".to_string()),
            author_name: "Alice".to_string(),
            author_username: "alice".to_string(),
            author_profile_image_url: Some("https://example.com/alice.jpg".to_string()),
            text: "hello".to_string(),
            created_at_ms,
            received_at_ms: created_at_ms,
            url: format!("https://x.com/alice/status/{id}"),
        }
    }
}
