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
// Keep poll payloads small because X API usage is cost-sensitive.
const X_FEED_FETCH_LIMIT: usize = 10;
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
pub(crate) struct XOAuthTokenRefresh {
    pub(crate) access_token: Zeroizing<String>,
    pub(crate) refresh_token: Option<Zeroizing<String>>,
    pub(crate) expires_in_secs: Option<u64>,
}

impl fmt::Debug for XOAuthTokenRefresh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XOAuthTokenRefresh")
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("expires_in_secs", &self.expires_in_secs)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XCredentialRequestKind {
    AuthContext,
    TokenRefresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XCredentialRequest {
    request_id: u64,
    kind: XCredentialRequestKind,
}

#[derive(Clone, PartialEq, Eq)]
struct XTokenRefreshRequest {
    owner: XCredentialRequest,
    oauth_client_id: SensitiveString,
    fallback_refresh_token: SensitiveString,
}

impl fmt::Debug for XTokenRefreshRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XTokenRefreshRequest")
            .field("owner", &self.owner)
            .field("oauth_client_id", &"<redacted>")
            .field("fallback_refresh_token", &"<redacted>")
            .finish()
    }
}

impl XTokenRefreshRequest {
    fn into_credentials(self) -> (Zeroizing<String>, Zeroizing<String>) {
        (
            self.oauth_client_id.into_zeroizing(),
            self.fallback_refresh_token.into_zeroizing(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct XFeedRequestAllocators {
    credential: u64,
    lists: u64,
    source: u64,
    profile_image: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XListsRequestKind {
    AuthContext { credential_request_id: u64 },
    ListsRefresh,
}

#[derive(Clone, PartialEq, Eq)]
struct XListsRequest {
    request_id: u64,
    kind: XListsRequestKind,
    user_id: Option<String>,
}

impl fmt::Debug for XListsRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XListsRequest")
            .field("request_id", &self.request_id)
            .field("kind", &self.kind)
            .field("user_id", &self.user_id.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct XSourceRefreshRequest {
    request_id: u64,
    source: XFeedSource,
    user_id: String,
}

impl fmt::Debug for XSourceRefreshRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XSourceRefreshRequest")
            .field("request_id", &self.request_id)
            .field("source", &self.source)
            .field("user_id", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct XProfileImageRequest {
    request_id: u64,
    profile_key: String,
    image_url: String,
}

impl fmt::Debug for XProfileImageRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XProfileImageRequest")
            .field("request_id", &self.request_id)
            .field("profile_key", &"<redacted>")
            .field("image_url", &"<url>")
            .finish()
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
            .field("id", &"<redacted>")
            .field("author_id", &self.author_id.as_ref().map(|_| "<redacted>"))
            .field("author_name", &"<redacted>")
            .field("author_username", &"<redacted>")
            .field(
                "author_profile_image_url",
                &self.author_profile_image_url.as_ref().map(|_| "<url>"),
            )
            .field("text", &"<redacted>")
            .field("created_at_ms", &"<redacted>")
            .field("received_at_ms", &"<redacted>")
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
            .field("newest_id", &self.newest_id.as_ref().map(|_| "<redacted>"))
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
            .field("message", &"<redacted>")
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
    pub(crate) oauth_client_id_input: SensitiveString,
    pub(crate) refresh_token_input: SensitiveString,
    pending_access_token: SensitiveString,
    pending_oauth_client_id: SensitiveString,
    pending_refresh_token: SensitiveString,
    access_token: SensitiveString,
    oauth_client_id: SensitiveString,
    refresh_token: SensitiveString,
    access_token_expires_at_ms: Option<u64>,
    pub(crate) auth_user: Option<XAuthenticatedUser>,
    pub(crate) lists: Vec<XListSummary>,
    next_credential_request_id: u64,
    credential_request: Option<XCredentialRequest>,
    token_refresh_request: Option<XTokenRefreshRequest>,
    next_lists_request_id: u64,
    lists_request: Option<XListsRequest>,
    next_source_refresh_request_id: u64,
    source_refresh_requests: HashMap<String, XSourceRefreshRequest>,
    source_rate_limit_reset_ms: HashMap<String, u64>,
    pub(crate) connecting: bool,
    pub(crate) token_refreshing: bool,
    pub(crate) lists_loading: bool,
    pub(crate) status: Option<(String, bool)>,
    pub(crate) instances: HashMap<XFeedId, XFeedInstance>,
    pub(crate) author_profiles: HashMap<String, XAuthorProfile>,
    next_profile_image_request_id: u64,
    profile_image_requests: HashMap<u64, XProfileImageRequest>,
}

impl fmt::Debug for XFeedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XFeedState")
            .field("access_token_input", &"<redacted>")
            .field("oauth_client_id_input", &"<redacted>")
            .field("refresh_token_input", &"<redacted>")
            .field("pending_access_token", &"<redacted>")
            .field("pending_oauth_client_id", &"<redacted>")
            .field("pending_refresh_token", &"<redacted>")
            .field("access_token", &"<redacted>")
            .field("oauth_client_id", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .field(
                "access_token_expires_at_ms",
                &self.access_token_expires_at_ms,
            )
            .field("auth_user", &self.auth_user)
            .field("lists", &self.lists.len())
            .field(
                "next_credential_request_id",
                &self.next_credential_request_id,
            )
            .field("credential_request", &self.credential_request)
            .field("token_refresh_request", &self.token_refresh_request)
            .field("next_lists_request_id", &self.next_lists_request_id)
            .field("lists_request", &self.lists_request)
            .field(
                "next_source_refresh_request_id",
                &self.next_source_refresh_request_id,
            )
            .field(
                "source_refresh_requests",
                &self.source_refresh_requests.len(),
            )
            .field(
                "source_rate_limit_reset_ms",
                &self.source_rate_limit_reset_ms.len(),
            )
            .field("connecting", &self.connecting)
            .field("token_refreshing", &self.token_refreshing)
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
            .field("profile_image_requests", &self.profile_image_requests.len())
            .finish()
    }
}

impl XFeedState {
    pub(crate) fn new(
        configs: &[crate::config::XFeedConfig],
        access_token: &str,
        oauth_client_id: &str,
        refresh_token: &str,
    ) -> Self {
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
            oauth_client_id_input: sensitive_string(String::new()),
            refresh_token_input: sensitive_string(String::new()),
            pending_access_token: sensitive_string(String::new()),
            pending_oauth_client_id: sensitive_string(String::new()),
            pending_refresh_token: sensitive_string(String::new()),
            access_token: sensitive_string(access_token),
            oauth_client_id: sensitive_string(oauth_client_id.trim().to_string()),
            refresh_token: sensitive_string(refresh_token.trim().to_string()),
            access_token_expires_at_ms: None,
            auth_user: None,
            lists: Vec::new(),
            next_credential_request_id: 0,
            credential_request: None,
            token_refresh_request: None,
            next_lists_request_id: 0,
            lists_request: None,
            next_source_refresh_request_id: 0,
            source_refresh_requests: HashMap::new(),
            source_rate_limit_reset_ms: HashMap::new(),
            connecting: false,
            token_refreshing: false,
            lists_loading: false,
            status: None,
            instances,
            author_profiles: HashMap::new(),
            next_profile_image_request_id: 0,
            profile_image_requests: HashMap::new(),
        }
    }

    pub(crate) fn has_access_token(&self) -> bool {
        !self.access_token.trim().is_empty()
    }

    pub(crate) fn has_refresh_credentials(&self) -> bool {
        !self.oauth_client_id.trim().is_empty() && !self.refresh_token.trim().is_empty()
    }

    pub(crate) fn has_refresh_credential_input(&self) -> bool {
        !self.oauth_client_id_input.trim().is_empty() || !self.refresh_token_input.trim().is_empty()
    }

    pub(crate) fn loading(&self) -> bool {
        self.connecting
            || self.token_refreshing
            || self.lists_loading
            || !self.source_refresh_requests.is_empty()
    }

    pub(crate) fn access_token_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.access_token.trim().to_string())
    }

    pub(crate) fn oauth_client_id_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.oauth_client_id.trim().to_string())
    }

    pub(crate) fn refresh_token_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.refresh_token.trim().to_string())
    }

    pub(crate) fn oauth_credentials_for_secret(
        &self,
    ) -> (Zeroizing<String>, Zeroizing<String>, Zeroizing<String>) {
        (
            Zeroizing::new(self.access_token.trim().to_string()),
            Zeroizing::new(self.oauth_client_id.trim().to_string()),
            Zeroizing::new(self.refresh_token.trim().to_string()),
        )
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

    pub(crate) fn refresh_credentials_candidate_from_input(
        &mut self,
    ) -> Option<(Zeroizing<String>, Zeroizing<String>)> {
        let client_id = self.oauth_client_id_input.trim().to_string();
        let refresh_token = self.refresh_token_input.trim().to_string();
        if client_id.is_empty() || refresh_token.is_empty() {
            self.status = Some((
                "Paste both an X OAuth 2.0 Client ID and refresh token".to_string(),
                true,
            ));
            return None;
        }

        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
        self.pending_oauth_client_id = sensitive_string(client_id.clone());
        self.pending_refresh_token = sensitive_string(refresh_token.clone());
        self.oauth_client_id_input.zeroize();
        self.refresh_token_input.zeroize();
        Some((Zeroizing::new(client_id), Zeroizing::new(refresh_token)))
    }

    pub(crate) fn commit_access_token(&mut self, token: &str) -> bool {
        let changed = self.set_oauth_credentials_from_secret(token, "", "", None);
        self.access_token_input.zeroize();
        self.pending_access_token.zeroize();
        self.oauth_client_id_input.zeroize();
        self.refresh_token_input.zeroize();
        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
        changed
    }

    pub(crate) fn commit_oauth_credentials(
        &mut self,
        access_token: &str,
        oauth_client_id: &str,
        refresh_token: &str,
        expires_at_ms: Option<u64>,
    ) -> bool {
        let changed = self.set_oauth_credentials_from_secret(
            access_token,
            oauth_client_id,
            refresh_token,
            expires_at_ms,
        );
        self.access_token_input.zeroize();
        self.oauth_client_id_input.zeroize();
        self.refresh_token_input.zeroize();
        self.pending_access_token.zeroize();
        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
        changed
    }

    pub(crate) fn pending_access_token_for_secret(&self) -> Option<Zeroizing<String>> {
        let token = self.pending_access_token.trim().to_string();
        (!token.is_empty()).then(|| Zeroizing::new(token))
    }

    pub(crate) fn clear_pending_access_token(&mut self) {
        self.pending_access_token.zeroize();
    }

    pub(crate) fn clear_pending_oauth_credentials(&mut self) {
        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
    }

    pub(crate) fn set_oauth_credentials_from_secret(
        &mut self,
        access_token: &str,
        oauth_client_id: &str,
        refresh_token: &str,
        expires_at_ms: Option<u64>,
    ) -> bool {
        let access_token = access_token.trim();
        let oauth_client_id = oauth_client_id.trim();
        let refresh_token = refresh_token.trim();
        let changed = self.access_token.trim() != access_token
            || self.oauth_client_id.trim() != oauth_client_id
            || self.refresh_token.trim() != refresh_token;
        if changed {
            self.invalidate_requests();
            self.auth_user = None;
            self.lists.clear();
            self.author_profiles.clear();
            self.connecting = false;
            self.token_refreshing = false;
            self.lists_loading = false;
            for instance in self.instances.values_mut() {
                if access_token.is_empty() || instance.source.is_private() {
                    instance.source = XFeedSource::Following;
                }
                instance.last_error = None;
                instance.posts.clear();
                instance.last_refresh_ms = None;
            }
        }

        self.access_token.zeroize();
        self.oauth_client_id.zeroize();
        self.refresh_token.zeroize();
        self.access_token = sensitive_string(access_token.to_string());
        self.oauth_client_id = sensitive_string(oauth_client_id.to_string());
        self.refresh_token = sensitive_string(refresh_token.to_string());
        self.access_token_expires_at_ms = expires_at_ms;
        changed
    }

    pub(crate) fn access_token_refresh_due(&self, now_ms: u64) -> bool {
        if !self.has_refresh_credentials() {
            return false;
        }
        match self.access_token_expires_at_ms {
            Some(expires_at_ms) => expires_at_ms.saturating_sub(now_ms) <= 60_000,
            None => true,
        }
    }

    pub(crate) fn clear_access_token(&mut self) {
        self.access_token_input.zeroize();
        self.oauth_client_id_input.zeroize();
        self.refresh_token_input.zeroize();
        self.pending_access_token.zeroize();
        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
        self.invalidate_requests();
        self.set_oauth_credentials_from_secret("", "", "", None);
        self.status = Some(("X token cleared".to_string(), false));
    }

    pub(crate) fn invalidate_requests(&mut self) {
        self.credential_request = None;
        self.token_refresh_request = None;
        self.lists_request = None;
        self.source_refresh_requests.clear();
        self.profile_image_requests.clear();
        self.connecting = false;
        self.token_refreshing = false;
        self.lists_loading = false;
        self.source_rate_limit_reset_ms.clear();
        for profile in self.author_profiles.values_mut() {
            profile.image_loading_url = None;
            profile.image_request_id = 0;
        }
    }

    fn begin_credential_request(&mut self, kind: XCredentialRequestKind) -> XCredentialRequest {
        self.next_credential_request_id = self.next_credential_request_id.wrapping_add(1);
        let request = XCredentialRequest {
            request_id: self.next_credential_request_id,
            kind,
        };
        self.credential_request = Some(request);
        request
    }

    /// Begin a read-only auth-context request. An in-flight token refresh stays
    /// authoritative because its response may rotate the refresh token.
    pub(crate) fn begin_auth_request(&mut self) -> Option<u64> {
        if self.token_refreshing {
            return None;
        }

        self.token_refresh_request = None;
        self.clear_pending_oauth_credentials();
        self.connecting = true;
        let request = self.begin_credential_request(XCredentialRequestKind::AuthContext);
        self.begin_auth_context_lists_request(request.request_id);
        Some(request.request_id)
    }

    /// Claim the exact auth request and report whether its bundled List result
    /// still owns List state. A newer explicit List refresh may supersede only
    /// that nested result without invalidating the authenticated user result.
    pub(crate) fn finish_auth_request(&mut self, request_id: u64) -> Option<bool> {
        let request = XCredentialRequest {
            request_id,
            kind: XCredentialRequestKind::AuthContext,
        };
        if self.credential_request != Some(request) {
            return None;
        }
        self.credential_request = None;
        self.connecting = false;
        Some(self.finish_auth_context_lists_request(request_id))
    }

    /// Begin a token refresh with immutable dispatch-time fallback credentials.
    /// This safely supersedes an older read-only auth-context request.
    pub(crate) fn begin_token_refresh_request(
        &mut self,
        oauth_client_id: &str,
        fallback_refresh_token: &str,
    ) -> Option<u64> {
        if self.token_refreshing {
            return None;
        }

        self.clear_pending_access_token();
        self.connecting = false;
        if let Some(auth_request) = self
            .credential_request
            .filter(|request| request.kind == XCredentialRequestKind::AuthContext)
        {
            self.cancel_auth_context_lists_request(auth_request.request_id);
        }
        let owner = self.begin_credential_request(XCredentialRequestKind::TokenRefresh);
        self.token_refresh_request = Some(XTokenRefreshRequest {
            owner,
            oauth_client_id: sensitive_string(oauth_client_id.to_string()),
            fallback_refresh_token: sensitive_string(fallback_refresh_token.to_string()),
        });
        self.token_refreshing = true;
        Some(owner.request_id)
    }

    pub(crate) fn finish_token_refresh_request(
        &mut self,
        request_id: u64,
    ) -> Option<(Zeroizing<String>, Zeroizing<String>)> {
        let request = XCredentialRequest {
            request_id,
            kind: XCredentialRequestKind::TokenRefresh,
        };
        if self.credential_request != Some(request)
            || !self
                .token_refresh_request
                .as_ref()
                .is_some_and(|context| context.owner == request)
        {
            return None;
        }

        self.credential_request = None;
        self.token_refreshing = false;
        self.token_refresh_request
            .take()
            .map(XTokenRefreshRequest::into_credentials)
    }

    pub(crate) fn request_allocators(&self) -> XFeedRequestAllocators {
        XFeedRequestAllocators {
            credential: self.next_credential_request_id,
            lists: self.next_lists_request_id,
            source: self.next_source_refresh_request_id,
            profile_image: self.next_profile_image_request_id,
        }
    }

    pub(crate) fn restore_request_allocators(&mut self, allocators: XFeedRequestAllocators) {
        self.next_credential_request_id = allocators.credential;
        self.next_lists_request_id = allocators.lists;
        self.next_source_refresh_request_id = allocators.source;
        self.next_profile_image_request_id = allocators.profile_image;
    }

    #[cfg(test)]
    pub(crate) fn credential_request_allocator(&self) -> u64 {
        self.next_credential_request_id
    }

    #[cfg(test)]
    pub(crate) fn current_auth_request_id(&self) -> Option<u64> {
        self.credential_request
            .filter(|request| request.kind == XCredentialRequestKind::AuthContext)
            .map(|request| request.request_id)
    }

    #[cfg(test)]
    pub(crate) fn current_token_refresh_request_id(&self) -> Option<u64> {
        self.credential_request
            .filter(|request| request.kind == XCredentialRequestKind::TokenRefresh)
            .map(|request| request.request_id)
    }

    fn allocate_lists_request_id(&mut self) -> u64 {
        loop {
            self.next_lists_request_id = self.next_lists_request_id.wrapping_add(1);
            let request_id = self.next_lists_request_id;
            if !self
                .lists_request
                .as_ref()
                .is_some_and(|request| request.request_id == request_id)
            {
                return request_id;
            }
        }
    }

    fn begin_auth_context_lists_request(&mut self, credential_request_id: u64) {
        let request_id = self.allocate_lists_request_id();
        self.lists_request = Some(XListsRequest {
            request_id,
            kind: XListsRequestKind::AuthContext {
                credential_request_id,
            },
            user_id: None,
        });
        self.lists_loading = false;
    }

    fn finish_auth_context_lists_request(&mut self, credential_request_id: u64) -> bool {
        if !self.lists_request.as_ref().is_some_and(|request| {
            matches!(
                request.kind,
                XListsRequestKind::AuthContext {
                    credential_request_id: owner_id
                } if owner_id == credential_request_id
            )
        }) {
            return false;
        }

        self.lists_request = None;
        true
    }

    fn cancel_auth_context_lists_request(&mut self, credential_request_id: u64) {
        let _ = self.finish_auth_context_lists_request(credential_request_id);
    }

    pub(crate) fn begin_lists_request(&mut self, user_id: &str) -> u64 {
        let request_id = self.allocate_lists_request_id();
        self.lists_request = Some(XListsRequest {
            request_id,
            kind: XListsRequestKind::ListsRefresh,
            user_id: Some(user_id.to_string()),
        });
        self.lists_loading = true;
        request_id
    }

    pub(crate) fn finish_lists_request(
        &mut self,
        request_id: u64,
        current_user_id: Option<&str>,
    ) -> bool {
        if !self.lists_request.as_ref().is_some_and(|request| {
            request.request_id == request_id && request.kind == XListsRequestKind::ListsRefresh
        }) {
            return false;
        }

        let request = self
            .lists_request
            .take()
            .expect("matching X Lists request must be present");
        self.lists_loading = false;
        current_user_id == request.user_id.as_deref()
    }

    fn allocate_source_refresh_request_id(&mut self) -> u64 {
        loop {
            self.next_source_refresh_request_id =
                self.next_source_refresh_request_id.wrapping_add(1);
            let request_id = self.next_source_refresh_request_id;
            if !self
                .source_refresh_requests
                .values()
                .any(|request| request.request_id == request_id)
            {
                return request_id;
            }
        }
    }

    pub(crate) fn begin_source_refresh(&mut self, source: &XFeedSource, user_id: &str) -> u64 {
        let request_id = self.allocate_source_refresh_request_id();
        self.source_refresh_requests.insert(
            source.key(),
            XSourceRefreshRequest {
                request_id,
                source: source.clone(),
                user_id: user_id.to_string(),
            },
        );
        request_id
    }

    pub(crate) fn finish_source_refresh(
        &mut self,
        source: &XFeedSource,
        request_id: u64,
        current_user_id: Option<&str>,
    ) -> bool {
        let key = source.key();
        if !self
            .source_refresh_requests
            .get(&key)
            .is_some_and(|request| request.request_id == request_id && &request.source == source)
        {
            return false;
        }

        let request = self
            .source_refresh_requests
            .remove(&key)
            .expect("matching X source request must be present");
        current_user_id == Some(request.user_id.as_str())
    }

    pub(crate) fn source_refresh_in_flight(&self, source: &XFeedSource) -> bool {
        self.source_refresh_requests.contains_key(&source.key())
    }

    fn allocate_profile_image_request_id(&mut self) -> u64 {
        loop {
            self.next_profile_image_request_id = self.next_profile_image_request_id.wrapping_add(1);
            let request_id = self.next_profile_image_request_id;
            if request_id != 0 && !self.profile_image_requests.contains_key(&request_id) {
                return request_id;
            }
        }
    }

    pub(crate) fn begin_profile_image_request(
        &mut self,
        profile_key: &str,
        image_url: &str,
    ) -> u64 {
        let request_id = self.allocate_profile_image_request_id();
        self.profile_image_requests.insert(
            request_id,
            XProfileImageRequest {
                request_id,
                profile_key: profile_key.to_string(),
                image_url: image_url.to_string(),
            },
        );
        request_id
    }

    pub(crate) fn cancel_profile_image_request(&mut self, request_id: u64) {
        if request_id != 0 {
            self.profile_image_requests.remove(&request_id);
        }
    }

    pub(crate) fn finish_profile_image_request(
        &mut self,
        request_id: u64,
    ) -> Option<(String, String)> {
        self.profile_image_requests
            .remove(&request_id)
            .map(|request| (request.profile_key, request.image_url))
    }

    #[cfg(test)]
    pub(crate) fn current_lists_request_id(&self) -> Option<u64> {
        self.lists_request
            .as_ref()
            .map(|request| request.request_id)
    }

    #[cfg(test)]
    pub(crate) fn current_source_refresh_request_id(&self, source: &XFeedSource) -> Option<u64> {
        self.source_refresh_requests
            .get(&source.key())
            .map(|request| request.request_id)
    }

    #[cfg(test)]
    pub(crate) fn set_noncredential_request_allocators_for_test(
        &mut self,
        lists: u64,
        source: u64,
        profile_image: u64,
    ) {
        self.next_lists_request_id = lists;
        self.next_source_refresh_request_id = source;
        self.next_profile_image_request_id = profile_image;
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
        .send()
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

#[derive(Deserialize)]
struct XMeResponse {
    data: XUserPayload,
}

impl fmt::Debug for XMeResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XMeResponse")
            .field("data", &self.data)
            .finish()
    }
}

#[derive(Deserialize)]
struct XListsResponse {
    data: Option<Vec<XListPayload>>,
}

impl fmt::Debug for XListsResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XListsResponse")
            .field(
                "data",
                &self.data.as_ref().map(|lists| ("<redacted>", lists.len())),
            )
            .finish()
    }
}

#[derive(Deserialize)]
struct XOAuthTokenPayload {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct XListPayload {
    id: String,
    name: String,
    private: Option<bool>,
}

impl fmt::Debug for XListPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XListPayload")
            .field("id", &"<redacted>")
            .field("name", &"<redacted>")
            .field("private", &self.private)
            .finish()
    }
}

#[derive(Deserialize)]
struct XTimelineResponse {
    data: Option<Vec<XTweetPayload>>,
    includes: Option<XTimelineIncludes>,
    meta: Option<XTimelineMeta>,
}

impl fmt::Debug for XTimelineResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XTimelineResponse")
            .field(
                "data",
                &self.data.as_ref().map(|posts| ("<redacted>", posts.len())),
            )
            .field("includes", &self.includes)
            .field("meta", &self.meta)
            .finish()
    }
}

#[derive(Deserialize)]
struct XTimelineIncludes {
    users: Option<Vec<XUserPayload>>,
}

impl fmt::Debug for XTimelineIncludes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XTimelineIncludes")
            .field(
                "users",
                &self.users.as_ref().map(|users| ("<redacted>", users.len())),
            )
            .finish()
    }
}

#[derive(Deserialize)]
struct XTimelineMeta {
    newest_id: Option<String>,
}

impl fmt::Debug for XTimelineMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XTimelineMeta")
            .field("newest_id", &self.newest_id.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Deserialize)]
struct XTweetPayload {
    id: String,
    text: String,
    author_id: Option<String>,
    created_at: Option<String>,
}

impl fmt::Debug for XTweetPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XTweetPayload")
            .field("id", &"<redacted>")
            .field("text", &"<redacted>")
            .field("author_id", &self.author_id.as_ref().map(|_| "<redacted>"))
            .field(
                "created_at",
                &self.created_at.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Deserialize)]
struct XUserPayload {
    id: String,
    username: String,
    name: String,
    profile_image_url: Option<String>,
}

impl fmt::Debug for XUserPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XUserPayload")
            .field("id", &"<redacted>")
            .field("username", &"<redacted>")
            .field("name", &"<redacted>")
            .field(
                "profile_image_url",
                &self.profile_image_url.as_ref().map(|_| "<url>"),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_feed_state_debug_redacts_tokens_and_status() {
        let mut state = XFeedState::new(&[], "", "", "");
        state.access_token_input = sensitive_string("token-input");
        state.oauth_client_id_input = sensitive_string("client-input");
        state.refresh_token_input = sensitive_string("refresh-input");
        state.access_token = sensitive_string("saved-token");
        state.oauth_client_id = sensitive_string("saved-client");
        state.refresh_token = sensitive_string("saved-refresh");
        state.status = Some(("auth_token=token-input failed".to_string(), true));
        let _request_id = state
            .begin_token_refresh_request("request-client", "request-refresh")
            .expect("token refresh owner");
        state.begin_lists_request("request-user");
        state.begin_source_refresh(
            &XFeedSource::List {
                id: "request-list-id".to_string(),
                name: "request-list-name".to_string(),
                private: true,
            },
            "request-source-user",
        );
        state.begin_profile_image_request(
            "request-profile-key",
            "https://images.invalid/request-profile.png",
        );

        let rendered = format!("{state:?}");

        assert!(!rendered.contains("token-input"));
        assert!(!rendered.contains("client-input"));
        assert!(!rendered.contains("refresh-input"));
        assert!(!rendered.contains("saved-token"));
        assert!(!rendered.contains("saved-client"));
        assert!(!rendered.contains("saved-refresh"));
        assert!(!rendered.contains("request-client"));
        assert!(!rendered.contains("request-refresh"));
        assert!(!rendered.contains("request-user"));
        assert!(!rendered.contains("request-list-id"));
        assert!(!rendered.contains("request-list-name"));
        assert!(!rendered.contains("request-source-user"));
        assert!(!rendered.contains("request-profile-key"));
        assert!(!rendered.contains("request-profile.png"));
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn direct_access_token_commit_clears_refresh_credentials() {
        let mut state = XFeedState::new(&[], "old-access", "old-client", "old-refresh");
        let refresh_request_id = state
            .begin_token_refresh_request("old-client", "old-refresh")
            .expect("token refresh owner");
        let request_allocator = state.credential_request_allocator();

        assert!(state.commit_access_token("new-access"));

        let (access_token, client_id, refresh_token) = state.oauth_credentials_for_secret();
        assert_eq!(access_token.as_str(), "new-access");
        assert_eq!(client_id.as_str(), "");
        assert_eq!(refresh_token.as_str(), "");
        assert!(!state.has_refresh_credentials());
        assert!(!state.token_refreshing);
        assert!(
            state
                .finish_token_refresh_request(refresh_request_id)
                .is_none()
        );
        assert_eq!(state.credential_request_allocator(), request_allocator);
    }

    #[test]
    fn clear_access_token_invalidates_pending_refresh_request() {
        let mut state = XFeedState::new(&[], "", "", "");
        state.pending_oauth_client_id = sensitive_string("pending-client");
        state.pending_refresh_token = sensitive_string("pending-refresh");
        let refresh_request_id = state
            .begin_token_refresh_request("pending-client", "pending-refresh")
            .expect("token refresh owner");
        let request_allocator = state.credential_request_allocator();

        state.clear_access_token();

        let (access_token, client_id, refresh_token) = state.oauth_credentials_for_secret();
        assert_eq!(access_token.as_str(), "");
        assert_eq!(client_id.as_str(), "");
        assert_eq!(refresh_token.as_str(), "");
        assert!(state.pending_oauth_client_id.is_empty());
        assert!(state.pending_refresh_token.is_empty());
        assert!(!state.token_refreshing);
        assert!(
            state
                .finish_token_refresh_request(refresh_request_id)
                .is_none()
        );
        assert_eq!(state.credential_request_allocator(), request_allocator);
    }

    #[test]
    fn credential_owner_wraps_and_settles_only_the_newest_request_once() {
        let mut state = XFeedState::new(&[], "", "", "");
        state.next_credential_request_id = u64::MAX - 1;

        let older_request_id = state.begin_auth_request().expect("older auth owner");
        let newer_request_id = state.begin_auth_request().expect("newer auth owner");

        assert_eq!(older_request_id, u64::MAX);
        assert_eq!(newer_request_id, 0);
        assert_eq!(state.finish_auth_request(older_request_id), None);
        assert_eq!(state.finish_auth_request(newer_request_id), Some(true));
        assert_eq!(state.finish_auth_request(newer_request_id), None);
    }

    #[test]
    fn source_and_profile_allocators_wrap_without_colliding_with_live_owners() {
        let mut state = XFeedState::new(&[], "", "", "");
        state.next_source_refresh_request_id = u64::MAX - 1;
        state.next_profile_image_request_id = u64::MAX - 1;
        let following = XFeedSource::Following;
        let list = XFeedSource::List {
            id: "list-a".to_string(),
            name: "List A".to_string(),
            private: false,
        };

        let following_request_id = state.begin_source_refresh(&following, "user-a");
        let list_request_id = state.begin_source_refresh(&list, "user-a");
        assert_eq!(following_request_id, u64::MAX);
        assert_eq!(list_request_id, 0);
        assert_ne!(following_request_id, list_request_id);

        assert!(state.finish_source_refresh(&following, following_request_id, Some("user-a")));
        let next_following_request_id = state.begin_source_refresh(&following, "user-a");
        assert_eq!(next_following_request_id, 1);
        assert_ne!(next_following_request_id, list_request_id);
        assert!(!state.finish_source_refresh(&following, following_request_id, Some("user-a")));
        assert!(state.finish_source_refresh(&following, next_following_request_id, Some("user-a")));
        assert!(!state.finish_source_refresh(
            &following,
            next_following_request_id,
            Some("user-a")
        ));

        let first_image_request_id =
            state.begin_profile_image_request("profile-a", "https://images.invalid/a.png");
        let second_image_request_id =
            state.begin_profile_image_request("profile-b", "https://images.invalid/b.png");
        assert_eq!(first_image_request_id, u64::MAX);
        assert_eq!(second_image_request_id, 1);
        assert_ne!(first_image_request_id, second_image_request_id);
        assert!(
            state
                .finish_profile_image_request(first_image_request_id)
                .is_some()
        );
        assert!(
            state
                .finish_profile_image_request(first_image_request_id)
                .is_none()
        );
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
        let mut state = XFeedState::new(&[], "", "", "");
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

    #[test]
    fn private_feed_models_keep_exact_values_out_of_debug_output() {
        const PRIVATE: &str = "private-x-value-sentinel";
        const PRIVATE_TIME: u64 = 9_876_543_210;
        let post = XFeedPost {
            id: PRIVATE.to_string(),
            author_id: Some(PRIVATE.to_string()),
            author_name: PRIVATE.to_string(),
            author_username: PRIVATE.to_string(),
            author_profile_image_url: Some(format!("https://images.invalid/{PRIVATE}.png")),
            text: PRIVATE.to_string(),
            created_at_ms: PRIVATE_TIME,
            received_at_ms: PRIVATE_TIME,
            url: format!("https://x.invalid/{PRIVATE}"),
        };
        let page = XFeedPage {
            source: XFeedSource::List {
                id: PRIVATE.to_string(),
                name: PRIVATE.to_string(),
                private: true,
            },
            posts: vec![post.clone()],
            newest_id: Some(PRIVATE.to_string()),
            rate_limited_until_ms: None,
        };
        let me = XMeResponse {
            data: XUserPayload {
                id: PRIVATE.to_string(),
                username: PRIVATE.to_string(),
                name: PRIVATE.to_string(),
                profile_image_url: Some(format!("https://images.invalid/{PRIVATE}.png")),
            },
        };
        let lists = XListsResponse {
            data: Some(vec![XListPayload {
                id: PRIVATE.to_string(),
                name: PRIVATE.to_string(),
                private: Some(true),
            }]),
        };
        let timeline = XTimelineResponse {
            data: Some(vec![XTweetPayload {
                id: PRIVATE.to_string(),
                text: PRIVATE.to_string(),
                author_id: Some(PRIVATE.to_string()),
                created_at: Some(PRIVATE.to_string()),
            }]),
            includes: Some(XTimelineIncludes {
                users: Some(vec![XUserPayload {
                    id: PRIVATE.to_string(),
                    username: PRIVATE.to_string(),
                    name: PRIVATE.to_string(),
                    profile_image_url: Some(format!("https://images.invalid/{PRIVATE}.png")),
                }]),
            }),
            meta: Some(XTimelineMeta {
                newest_id: Some(PRIVATE.to_string()),
            }),
        };

        for rendered in [
            format!("{post:?}"),
            format!("{page:?}"),
            format!("{me:?}"),
            format!("{lists:?}"),
            format!("{timeline:?}"),
            format!(
                "{:?}",
                XListPayload {
                    id: PRIVATE.to_string(),
                    name: PRIVATE.to_string(),
                    private: Some(true),
                }
            ),
            format!(
                "{:?}",
                XTimelineIncludes {
                    users: Some(vec![XUserPayload {
                        id: PRIVATE.to_string(),
                        username: PRIVATE.to_string(),
                        name: PRIVATE.to_string(),
                        profile_image_url: Some(format!("https://images.invalid/{PRIVATE}.png")),
                    }]),
                }
            ),
            format!(
                "{:?}",
                XTimelineMeta {
                    newest_id: Some(PRIVATE.to_string()),
                }
            ),
            format!(
                "{:?}",
                XTweetPayload {
                    id: PRIVATE.to_string(),
                    text: PRIVATE.to_string(),
                    author_id: Some(PRIVATE.to_string()),
                    created_at: Some(PRIVATE.to_string()),
                }
            ),
            format!(
                "{:?}",
                XUserPayload {
                    id: PRIVATE.to_string(),
                    username: PRIVATE.to_string(),
                    name: PRIVATE.to_string(),
                    profile_image_url: Some(format!("https://images.invalid/{PRIVATE}.png")),
                }
            ),
        ] {
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(PRIVATE), "{rendered}");
            assert!(!rendered.contains(&PRIVATE_TIME.to_string()), "{rendered}");
        }
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
