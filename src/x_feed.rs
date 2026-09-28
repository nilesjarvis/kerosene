mod client;

pub(crate) use client::{
    fetch_x_auth_context, fetch_x_feed_page, fetch_x_lists, fetch_x_profile_image_bytes,
    refresh_x_access_token,
};

use crate::app_state::{SensitiveString, sensitive_string};
use crate::helpers::{fallback_initials, redact_sensitive_response_text};
use iced::widget::image::Handle as ImageHandle;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use zeroize::{Zeroize, Zeroizing};

pub(crate) const X_FEED_REFRESH_INTERVAL_SECS: u64 = 10;
pub(crate) const X_FEED_POST_LIMIT: usize = 100;
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
    pub(crate) connect_request_id: u64,
    pub(crate) token_refresh_request_id: u64,
    pub(crate) lists_request_id: u64,
    pub(crate) refresh_request_id: u64,
    source_refresh_request_ids: HashMap<String, u64>,
    source_rate_limit_reset_ms: HashMap<String, u64>,
    pub(crate) connecting: bool,
    pub(crate) token_refreshing: bool,
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
            .field("connect_request_id", &self.connect_request_id)
            .field("token_refresh_request_id", &self.token_refresh_request_id)
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
            connect_request_id: 0,
            token_refresh_request_id: 0,
            lists_request_id: 0,
            refresh_request_id: 0,
            source_refresh_request_ids: HashMap::new(),
            source_rate_limit_reset_ms: HashMap::new(),
            connecting: false,
            token_refreshing: false,
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
            || !self.source_refresh_request_ids.is_empty()
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
            self.access_token_for_task(),
            self.oauth_client_id_for_task(),
            self.refresh_token_for_task(),
        )
    }

    pub(crate) fn access_token_candidate_from_input(&mut self) -> Option<Zeroizing<String>> {
        let token = self.access_token_input.trim();
        if token.is_empty() {
            self.status = Some(("Paste an X OAuth 2.0 user access token".to_string(), true));
            return None;
        }

        let token = Zeroizing::new(token.to_string());
        self.pending_access_token.zeroize();
        self.pending_access_token = token.clone().into();
        self.access_token_input.zeroize();
        Some(token)
    }

    pub(crate) fn refresh_credentials_candidate_from_input(
        &mut self,
    ) -> Option<(Zeroizing<String>, Zeroizing<String>)> {
        let client_id = self.oauth_client_id_input.trim();
        let refresh_token = self.refresh_token_input.trim();
        if client_id.is_empty() || refresh_token.is_empty() {
            self.status = Some((
                "Paste both an X OAuth 2.0 Client ID and refresh token".to_string(),
                true,
            ));
            return None;
        }

        let client_id = Zeroizing::new(client_id.to_string());
        let refresh_token = Zeroizing::new(refresh_token.to_string());
        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
        self.pending_oauth_client_id = client_id.clone().into();
        self.pending_refresh_token = refresh_token.clone().into();
        self.oauth_client_id_input.zeroize();
        self.refresh_token_input.zeroize();
        Some((client_id, refresh_token))
    }

    pub(crate) fn commit_access_token(&mut self, token: &str) -> bool {
        self.commit_oauth_credentials(token, "", "", None)
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
        self.clear_credential_inputs();
        changed
    }

    pub(crate) fn pending_access_token_for_secret(&self) -> Option<Zeroizing<String>> {
        let token = self.pending_access_token.trim();
        (!token.is_empty()).then(|| Zeroizing::new(token.to_string()))
    }

    pub(crate) fn pending_oauth_credentials_for_secret(
        &self,
    ) -> Option<(Zeroizing<String>, Zeroizing<String>)> {
        let client_id = self.pending_oauth_client_id.trim();
        let refresh_token = self.pending_refresh_token.trim();
        (!client_id.is_empty() && !refresh_token.is_empty()).then(|| {
            (
                Zeroizing::new(client_id.to_string()),
                Zeroizing::new(refresh_token.to_string()),
            )
        })
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
        self.clear_credential_inputs();
        self.invalidate_requests();
        self.set_oauth_credentials_from_secret("", "", "", None);
        self.status = Some(("X token cleared".to_string(), false));
    }

    /// Erase both editable fields and pending login credentials.
    fn clear_credential_inputs(&mut self) {
        self.access_token_input.zeroize();
        self.oauth_client_id_input.zeroize();
        self.refresh_token_input.zeroize();
        self.pending_access_token.zeroize();
        self.pending_oauth_client_id.zeroize();
        self.pending_refresh_token.zeroize();
    }

    pub(crate) fn invalidate_requests(&mut self) {
        self.connect_request_id = self.connect_request_id.saturating_add(1);
        self.token_refresh_request_id = self.token_refresh_request_id.saturating_add(1);
        self.lists_request_id = self.lists_request_id.saturating_add(1);
        self.refresh_request_id = self.refresh_request_id.saturating_add(1);
        self.token_refreshing = false;
        self.source_refresh_request_ids.clear();
        self.source_rate_limit_reset_ms.clear();
    }

    pub(crate) fn next_connect_request_id(&mut self) -> u64 {
        self.connect_request_id = self.connect_request_id.saturating_add(1);
        self.connect_request_id
    }

    pub(crate) fn next_token_refresh_request_id(&mut self) -> u64 {
        self.token_refresh_request_id = self.token_refresh_request_id.saturating_add(1);
        self.token_refresh_request_id
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
        let mut lists = self.lists.iter().collect::<Vec<_>>();
        lists.sort_by_cached_key(|list| (list.name.to_ascii_lowercase(), list.id.as_str()));
        for list in lists {
            if seen_lists.insert(list.id.as_str()) {
                options.push(XFeedSourceOption::new(XFeedSource::List {
                    id: list.id.clone(),
                    name: list.name.clone(),
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

    pub(crate) fn newest_seen_id(&self) -> Option<&str> {
        self.posts
            .iter()
            .filter_map(|post| post.id.parse::<u64>().ok().map(|id| (id, post.id.as_str())))
            .max_by_key(|(id, _)| *id)
            .map(|(_, id)| id)
    }
}

fn x_author_profile_key(author_id: Option<&str>, username: &str) -> String {
    author_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(|id| format!("id:{id}"))
        .unwrap_or_else(|| format!("username:{}", username.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests;
