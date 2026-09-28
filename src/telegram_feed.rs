mod client;

pub(crate) use client::{
    fetch_telegram_avatar_bytes, fetch_telegram_channel_posts, fetch_telegram_media_bytes,
};

use crate::api::ExchangeSymbol;
use crate::app_state::{SensitiveString, sensitive_string};
use crate::app_time::cooldown_heat;
use crate::helpers::{
    fallback_initials, format_seen_latency_label, positive_percent_change,
    redact_sensitive_response_text,
};
use crate::symbol_mentions::{SymbolAliasSource, SymbolMention, SymbolMentionResolver};
use iced::widget::image::Handle as ImageHandle;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;

pub(crate) const TELEGRAM_AVATAR_RETRY_BACKOFF_MS: u64 = 300_000;
pub(crate) const TELEGRAM_MEDIA_RETRY_BACKOFF_MS: u64 = 300_000;
pub(crate) const TELEGRAM_FEED_FETCH_LIMIT: usize = 10;
pub(crate) const TELEGRAM_FEED_RENDER_LIMIT: usize = 100;
pub(crate) const TELEGRAM_FEED_MAX_PUBLIC_CHANNELS: usize = 12;
pub(crate) const TELEGRAM_FEED_REFRESH_INTERVAL_SECS: u64 = 15;
pub(crate) const TELEGRAM_NEW_MESSAGE_COOLDOWN_MS: u64 = 120_000;
pub(crate) const TELEGRAM_FAST_HEALTH_CHECK_INTERVAL_SECS: u64 = 30;
pub(crate) const TELEGRAM_FAST_STALE_AFTER_MS: u64 =
    TELEGRAM_FAST_HEALTH_CHECK_INTERVAL_SECS * 3 * 1_000;
const TELEGRAM_FEED_SEEN_ID_LIMIT_PER_CHANNEL: usize = 1024;
const TELEGRAM_PRIVATE_CHANNEL_KEY_PREFIX: &str = "private:";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TelegramTickerMention {
    pub(crate) symbol: String,
    pub(crate) ticker: String,
    pub(crate) matched_text: String,
    pub(crate) source: SymbolAliasSource,
    pub(crate) confidence: u8,
    pub(crate) reference_price: Option<f64>,
    pub(crate) reference_seen_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TelegramMediaKind {
    Photo,
    Video,
    Sticker,
    Gif,
}

impl TelegramMediaKind {
    /// Short label shown when the preview image has not loaded (or failed). Also
    /// used as the post's body fallback for media-only messages.
    pub(crate) fn placeholder_label(self) -> &'static str {
        match self {
            Self::Photo => "[photo]",
            Self::Video => "[video]",
            Self::Sticker => "[sticker]",
            Self::Gif => "[gif]",
        }
    }
}

/// A single attached preview image for a post. The decoded `handle` is held only
/// in memory (never persisted): public-mode media is fetched from `url`
/// avatar-style, while fast-mode media is downloaded inline and arrives with the
/// handle already populated (and `url` left `None`).
#[derive(Clone, PartialEq)]
pub(crate) struct TelegramPostMedia {
    pub(crate) kind: TelegramMediaKind,
    pub(crate) url: Option<String>,
    pub(crate) handle: Option<ImageHandle>,
    pub(crate) loading_url: Option<String>,
    pub(crate) request_id: u64,
    pub(crate) failed_at_ms: Option<u64>,
}

impl fmt::Debug for TelegramPostMedia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // URLs can reference private channels, so summarize rather than print them.
        f.debug_struct("TelegramPostMedia")
            .field("kind", &self.kind)
            .field("url", &self.url.as_ref().map(|_| "<url>"))
            .field("handle", &self.handle.as_ref().map(|_| "<image>"))
            .field("loading_url", &self.loading_url.as_ref().map(|_| "<url>"))
            .field("request_id", &self.request_id)
            .field("failed_at_ms", &self.failed_at_ms)
            .finish()
    }
}

impl TelegramPostMedia {
    /// Public-mode descriptor: the preview must still be fetched from `url`.
    pub(crate) fn from_url(kind: TelegramMediaKind, url: String) -> Self {
        Self {
            kind,
            url: Some(url),
            handle: None,
            loading_url: None,
            request_id: 0,
            failed_at_ms: None,
        }
    }

    /// Fast-mode descriptor whose preview download is still pending; the card
    /// shows the kind label until the handle is filled in by a later event.
    pub(crate) fn placeholder(kind: TelegramMediaKind) -> Self {
        Self {
            kind,
            url: None,
            handle: None,
            loading_url: None,
            request_id: 0,
            failed_at_ms: None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct TelegramFeedPost {
    pub(crate) channel: String,
    pub(crate) message_id: u64,
    pub(crate) text: String,
    pub(crate) timestamp_ms: u64,
    pub(crate) source: TelegramFeedPostSource,
    pub(crate) received_at_ms: u64,
    pub(crate) applied_at_ms: u64,
    pub(crate) fetched_at_ms: u64,
    pub(crate) request_started_ms: u64,
    pub(crate) request_duration_ms: u64,
    pub(crate) first_seen_ms: u64,
    pub(crate) url: String,
    pub(crate) ticker_mentions: Vec<TelegramTickerMention>,
    pub(crate) media: Option<TelegramPostMedia>,
}

impl fmt::Debug for TelegramFeedPost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramFeedPost")
            .field(
                "channel",
                &redacted_telegram_channel_debug_value(&self.channel),
            )
            .field("message_id", &self.message_id)
            .field("text", &"<redacted>")
            .field("timestamp_ms", &self.timestamp_ms)
            .field("source", &self.source)
            .field("received_at_ms", &self.received_at_ms)
            .field("applied_at_ms", &self.applied_at_ms)
            .field("fetched_at_ms", &self.fetched_at_ms)
            .field("request_started_ms", &self.request_started_ms)
            .field("request_duration_ms", &self.request_duration_ms)
            .field("first_seen_ms", &self.first_seen_ms)
            .field("url", &redacted_telegram_url_debug_value(&self.url))
            .field("ticker_mentions", &self.ticker_mentions.len())
            .field("media", &self.media)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TelegramFeedPostSource {
    PublicPoll,
    FastBackfill,
    FastLive,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TelegramChannelProfile {
    pub(crate) channel: String,
    pub(crate) title: String,
    pub(crate) initials: String,
    pub(crate) avatar_url: Option<String>,
    pub(crate) avatar_handle: Option<ImageHandle>,
    pub(crate) avatar_loading_url: Option<String>,
    pub(crate) avatar_request_id: u64,
    pub(crate) avatar_failed_at_ms: Option<u64>,
}

impl fmt::Debug for TelegramChannelProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private = telegram_private_channel_peer_id_from_key(&self.channel).is_some();
        f.debug_struct("TelegramChannelProfile")
            .field(
                "channel",
                &redacted_telegram_channel_debug_value(&self.channel),
            )
            .field(
                "title",
                &redacted_private_telegram_debug_value(private, &self.title),
            )
            .field(
                "initials",
                &redacted_private_telegram_debug_value(private, &self.initials),
            )
            .field(
                "avatar_url",
                &self
                    .avatar_url
                    .as_ref()
                    .map(|value| redacted_private_telegram_debug_value(private, value)),
            )
            .field(
                "avatar_handle",
                &self.avatar_handle.as_ref().map(|_| "<image>"),
            )
            .field(
                "avatar_loading_url",
                &self
                    .avatar_loading_url
                    .as_ref()
                    .map(|value| redacted_private_telegram_debug_value(private, value)),
            )
            .field("avatar_request_id", &self.avatar_request_id)
            .field("avatar_failed_at_ms", &self.avatar_failed_at_ms)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TelegramFeedPage {
    pub(crate) profile: TelegramChannelProfile,
    pub(crate) posts: Vec<TelegramFeedPost>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TelegramFastAuthStage {
    Idle,
    CodeRequested,
    PasswordRequired,
    SignedIn,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum TelegramFastAuthOutcome {
    CodeSent,
    PasswordRequired { hint: Option<String> },
    SignedIn { display_name: String },
    SignedOut { warning: Option<String> },
}

impl fmt::Debug for TelegramFastAuthOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CodeSent => f.write_str("CodeSent"),
            Self::PasswordRequired { hint } => f
                .debug_struct("PasswordRequired")
                .field("hint", &hint.as_ref().map(|_| "<redacted>"))
                .finish(),
            Self::SignedIn { .. } => f
                .debug_struct("SignedIn")
                .field("display_name", &"<redacted>")
                .finish(),
            Self::SignedOut { warning } => f
                .debug_struct("SignedOut")
                .field("warning", &warning.as_ref().map(|_| "<redacted>"))
                .finish(),
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum TelegramFastFeedEvent {
    Status {
        connected: bool,
        auth_required: bool,
        message: String,
    },
    Loaded(String, Box<Result<TelegramFeedPage, String>>),
}

impl fmt::Debug for TelegramFastFeedEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status {
                connected,
                auth_required,
                message,
            } => f
                .debug_struct("Status")
                .field("connected", connected)
                .field("auth_required", auth_required)
                .field("message", &redact_sensitive_response_text(message))
                .finish(),
            Self::Loaded(channel, result) => {
                let result_summary = match result.as_ref() {
                    Ok(page) => format!(
                        "Ok(TelegramFeedPage {{ channel: {}, posts: {} }})",
                        redacted_telegram_channel_debug_value(&page.profile.channel),
                        page.posts.len()
                    ),
                    Err(error) => format!("Err({})", redact_sensitive_response_text(error)),
                };
                f.debug_tuple("Loaded")
                    .field(&redacted_telegram_channel_debug_value(channel))
                    .field(&result_summary)
                    .finish()
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TelegramFeedPrivateChannelConfig {
    pub peer_id: i64,
    pub title: String,
}

impl fmt::Debug for TelegramFeedPrivateChannelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramFeedPrivateChannelConfig")
            .field("peer_id", &"<redacted>")
            .field("title", &"<redacted>")
            .finish()
    }
}

impl TelegramFeedPrivateChannelConfig {
    pub(crate) fn normalized(&self) -> Option<Self> {
        (self.peer_id > 0).then(|| Self {
            peer_id: self.peer_id,
            title: normalize_private_channel_title(self.title.as_str(), self.peer_id),
        })
    }

    pub(crate) fn key(&self) -> String {
        telegram_private_channel_key(self.peer_id)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TelegramPrivateChannelCandidate {
    pub(crate) peer_id: i64,
    pub(crate) title: String,
    pub(crate) avatar_handle: Option<ImageHandle>,
}

impl fmt::Debug for TelegramPrivateChannelCandidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramPrivateChannelCandidate")
            .field("peer_id", &"<redacted>")
            .field("title", &"<redacted>")
            .field(
                "avatar_handle",
                &self.avatar_handle.as_ref().map(|_| "<image>"),
            )
            .finish()
    }
}

impl TelegramPrivateChannelCandidate {
    pub(crate) fn to_config(&self) -> TelegramFeedPrivateChannelConfig {
        TelegramFeedPrivateChannelConfig {
            peer_id: self.peer_id,
            title: self.title.clone(),
        }
    }

    pub(crate) fn to_profile(&self) -> TelegramChannelProfile {
        let channel = telegram_private_channel_key(self.peer_id);
        TelegramChannelProfile {
            initials: fallback_initials(&self.title, &channel),
            channel,
            title: self.title.clone(),
            avatar_url: None,
            avatar_handle: self.avatar_handle.clone(),
            avatar_loading_url: None,
            avatar_request_id: 0,
            avatar_failed_at_ms: None,
        }
    }
}

impl TelegramFeedPost {
    fn with_fetch_timing(
        mut self,
        request_started_ms: u64,
        fetched_at_ms: u64,
        request_duration_ms: u64,
    ) -> Self {
        self.request_started_ms = request_started_ms;
        self.fetched_at_ms = fetched_at_ms;
        self.received_at_ms = fetched_at_ms;
        self.request_duration_ms = request_duration_ms;
        self
    }

    pub(crate) fn mark_applied(&mut self, applied_at_ms: u64) {
        self.applied_at_ms = applied_at_ms;
    }
}

#[derive(Clone)]
pub(crate) struct TelegramFeedState {
    pub(crate) channels: Vec<String>,
    pub(crate) private_channels: Vec<TelegramFeedPrivateChannelConfig>,
    pub(crate) private_channel_candidates: Vec<TelegramPrivateChannelCandidate>,
    pub(crate) private_channel_candidates_loading: bool,
    pub(crate) private_channel_candidates_request_id: u64,
    pub(crate) private_channel_candidates_expanded: bool,
    pub(crate) notifications_enabled: bool,
    pub(crate) include_outcome_markets: bool,
    // Whether the user has left the Connect onboarding screen (by signing in or
    // choosing public mode). Persisted so onboarding only greets a user once.
    pub(crate) onboarding_dismissed: bool,
    pub(crate) fast_mode_enabled: bool,
    pub(crate) fast_api_id: Option<i32>,
    pub(crate) fast_api_id_input: String,
    pub(crate) fast_api_hash_input: SensitiveString,
    // Reveals the "use my own API credentials" inputs on the sign-in screen.
    pub(crate) fast_advanced_expanded: bool,
    // Dialing code shown beside the phone field, combined with `fast_phone_input`
    // when a login code is requested.
    pub(crate) fast_country_code: String,
    pub(crate) fast_phone_input: String,
    // When the most recent login code was requested, driving the resend cooldown.
    pub(crate) fast_code_sent_at_ms: Option<u64>,
    pub(crate) fast_code_input: SensitiveString,
    pub(crate) fast_password_input: SensitiveString,
    pub(crate) fast_auth_stage: TelegramFastAuthStage,
    pub(crate) fast_auth_request_id: u64,
    pub(crate) fast_auth_in_flight: bool,
    pub(crate) fast_connected: bool,
    pub(crate) fast_status: Option<(String, bool)>,
    pub(crate) fast_password_hint: Option<String>,
    pub(crate) fast_reconnect_nonce: u64,
    pub(crate) fast_last_event_ms: Option<u64>,
    // Whether the live-feed channel chip list is expanded. Runtime-only; defaults
    // to collapsed so a long channel list does not dominate the pane.
    pub(crate) channels_expanded: bool,
    pub(crate) channel_input: String,
    pub(crate) channel_profiles: HashMap<String, TelegramChannelProfile>,
    pub(crate) posts: Vec<TelegramFeedPost>,
    ticker_mention_resolver: SymbolMentionResolver,
    seen_post_ids: HashMap<String, VecDeque<u64>>,
    channel_refresh_request_ids: HashMap<String, u64>,
    next_channel_refresh_request_id: u64,
    pub(crate) loading_channels: Vec<String>,
    pub(crate) background_loading_channels: Vec<String>,
    pub(crate) next_avatar_request_id: u64,
    pub(crate) next_media_request_id: u64,
    pub(crate) last_error: Option<String>,
    pub(crate) last_refresh_ms: Option<u64>,
}

impl fmt::Debug for TelegramFeedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fast_status = self
            .fast_status
            .as_ref()
            .map(|(_message, is_error)| ("<redacted>", *is_error));
        f.debug_struct("TelegramFeedState")
            .field("channels", &self.channels)
            .field("private_channels", &self.private_channels)
            .field(
                "private_channel_candidates",
                &self.private_channel_candidates,
            )
            .field(
                "private_channel_candidates_loading",
                &self.private_channel_candidates_loading,
            )
            .field(
                "private_channel_candidates_request_id",
                &self.private_channel_candidates_request_id,
            )
            .field(
                "private_channel_candidates_expanded",
                &self.private_channel_candidates_expanded,
            )
            .field("notifications_enabled", &self.notifications_enabled)
            .field("include_outcome_markets", &self.include_outcome_markets)
            .field("onboarding_dismissed", &self.onboarding_dismissed)
            .field("fast_mode_enabled", &self.fast_mode_enabled)
            .field("fast_api_id", &"<redacted>")
            .field("fast_api_id_input", &"<redacted>")
            .field("fast_api_hash_input", &"<redacted>")
            .field("fast_advanced_expanded", &self.fast_advanced_expanded)
            .field("fast_country_code", &self.fast_country_code)
            .field("fast_phone_input", &"<redacted>")
            .field("fast_code_sent_at_ms", &self.fast_code_sent_at_ms)
            .field("fast_code_input", &"<redacted>")
            .field("fast_password_input", &"<redacted>")
            .field("fast_auth_stage", &self.fast_auth_stage)
            .field("fast_auth_request_id", &self.fast_auth_request_id)
            .field("fast_auth_in_flight", &self.fast_auth_in_flight)
            .field("fast_connected", &self.fast_connected)
            .field("fast_status", &fast_status)
            .field(
                "fast_password_hint",
                &self.fast_password_hint.as_ref().map(|_| "<redacted>"),
            )
            .field("fast_reconnect_nonce", &self.fast_reconnect_nonce)
            .field("fast_last_event_ms", &self.fast_last_event_ms)
            .field("channels_expanded", &self.channels_expanded)
            .field(
                "channel_input",
                &redacted_telegram_channel_debug_value(&self.channel_input),
            )
            .field(
                "channel_profiles",
                &TelegramChannelProfileMapDebug(&self.channel_profiles),
            )
            .field("posts", &TelegramPostListDebug(&self.posts))
            .field(
                "seen_post_ids",
                &TelegramSeenPostIdsDebug(&self.seen_post_ids),
            )
            .field(
                "channel_refresh_request_ids",
                &TelegramRefreshRequestIdsDebug(&self.channel_refresh_request_ids),
            )
            .field(
                "next_channel_refresh_request_id",
                &self.next_channel_refresh_request_id,
            )
            .field(
                "loading_channels",
                &TelegramChannelListDebug(&self.loading_channels),
            )
            .field(
                "background_loading_channels",
                &TelegramChannelListDebug(&self.background_loading_channels),
            )
            .field("next_avatar_request_id", &self.next_avatar_request_id)
            .field("next_media_request_id", &self.next_media_request_id)
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

struct TelegramChannelListDebug<'a>(&'a [String]);

impl fmt::Debug for TelegramChannelListDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(
                self.0
                    .iter()
                    .map(|channel| redacted_telegram_channel_debug_value(channel)),
            )
            .finish()
    }
}

struct TelegramChannelProfileMapDebug<'a>(&'a HashMap<String, TelegramChannelProfile>);

impl fmt::Debug for TelegramChannelProfileMapDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private = self
            .0
            .keys()
            .filter(|channel| telegram_private_channel_peer_id_from_key(channel).is_some())
            .count();
        f.debug_struct("TelegramChannelProfiles")
            .field("total", &self.0.len())
            .field("private", &private)
            .finish()
    }
}

struct TelegramPostListDebug<'a>(&'a [TelegramFeedPost]);

impl fmt::Debug for TelegramPostListDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private = self
            .0
            .iter()
            .filter(|post| telegram_private_channel_peer_id_from_key(&post.channel).is_some())
            .count();
        f.debug_struct("TelegramPosts")
            .field("total", &self.0.len())
            .field("private", &private)
            .finish()
    }
}

struct TelegramSeenPostIdsDebug<'a>(&'a HashMap<String, VecDeque<u64>>);

impl fmt::Debug for TelegramSeenPostIdsDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private = self
            .0
            .keys()
            .filter(|channel| telegram_private_channel_peer_id_from_key(channel).is_some())
            .count();
        let seen_ids = self.0.values().map(VecDeque::len).sum::<usize>();
        f.debug_struct("TelegramSeenPostIds")
            .field("channels", &self.0.len())
            .field("private_channels", &private)
            .field("seen_ids", &seen_ids)
            .finish()
    }
}

struct TelegramRefreshRequestIdsDebug<'a>(&'a HashMap<String, u64>);

impl fmt::Debug for TelegramRefreshRequestIdsDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private = self
            .0
            .keys()
            .filter(|channel| telegram_private_channel_peer_id_from_key(channel).is_some())
            .count();
        f.debug_struct("TelegramRefreshRequestIds")
            .field("channels", &self.0.len())
            .field("private_channels", &private)
            .finish()
    }
}

fn redacted_telegram_channel_debug_value(value: &str) -> &str {
    if telegram_private_channel_peer_id_from_key(value).is_some() {
        "<private>"
    } else {
        value
    }
}

fn redacted_telegram_url_debug_value(value: &str) -> &str {
    if value.contains("t.me/c/") || value.contains("telegram.me/c/") {
        "<private>"
    } else {
        value
    }
}

fn redacted_private_telegram_debug_value(private: bool, value: &str) -> &str {
    if private { "<redacted>" } else { value }
}

impl TelegramFeedState {
    pub(crate) fn new(
        channels: &[String],
        private_channels: &[TelegramFeedPrivateChannelConfig],
        notifications_enabled: bool,
        fast_mode_enabled: bool,
        fast_api_id: Option<i32>,
        include_outcome_markets: bool,
        onboarding_dismissed: bool,
    ) -> Self {
        let (channels, public_channels_capped) = normalized_channel_list_with_status(channels);
        Self {
            channels,
            private_channels: normalized_private_channel_list(private_channels),
            private_channel_candidates: Vec::new(),
            private_channel_candidates_loading: false,
            private_channel_candidates_request_id: 0,
            private_channel_candidates_expanded: false,
            notifications_enabled,
            include_outcome_markets,
            onboarding_dismissed,
            fast_mode_enabled,
            fast_api_id,
            fast_api_id_input: fast_api_id
                .map(|api_id| api_id.to_string())
                .unwrap_or_default(),
            fast_api_hash_input: sensitive_string(String::new()),
            fast_advanced_expanded: false,
            fast_country_code: default_telegram_country_code(),
            fast_phone_input: String::new(),
            fast_code_sent_at_ms: None,
            fast_code_input: sensitive_string(String::new()),
            fast_password_input: sensitive_string(String::new()),
            fast_auth_stage: TelegramFastAuthStage::Idle,
            fast_auth_request_id: 0,
            fast_auth_in_flight: false,
            fast_connected: false,
            fast_status: None,
            fast_password_hint: None,
            fast_reconnect_nonce: 0,
            fast_last_event_ms: None,
            channels_expanded: false,
            channel_input: String::new(),
            channel_profiles: HashMap::new(),
            posts: Vec::new(),
            ticker_mention_resolver: SymbolMentionResolver::empty(),
            seen_post_ids: HashMap::new(),
            channel_refresh_request_ids: HashMap::new(),
            next_channel_refresh_request_id: 0,
            loading_channels: Vec::new(),
            background_loading_channels: Vec::new(),
            next_avatar_request_id: 0,
            next_media_request_id: 0,
            last_error: public_channels_capped.then(|| {
                format!(
                    "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels; extra saved channels were ignored"
                )
            }),
            last_refresh_ms: None,
        }
    }

    pub(crate) fn loading(&self) -> bool {
        !self.loading_channels.is_empty()
    }

    pub(crate) fn channel_refresh_in_flight(&self) -> bool {
        self.loading() || !self.background_loading_channels.is_empty()
    }

    pub(crate) fn begin_channel_refresh(&mut self, channel: &str) -> u64 {
        self.next_channel_refresh_request_id =
            self.next_channel_refresh_request_id.saturating_add(1);
        let request_id = self.next_channel_refresh_request_id;
        self.channel_refresh_request_ids
            .insert(channel.to_string(), request_id);
        request_id
    }

    pub(crate) fn finish_channel_refresh(&mut self, channel: &str, request_id: u64) -> bool {
        if self
            .channel_refresh_request_ids
            .get(channel)
            .is_some_and(|current_id| *current_id == request_id)
        {
            self.channel_refresh_request_ids.remove(channel);
            return true;
        }

        false
    }

    pub(crate) fn clear_channel_refresh(&mut self, channel: &str) {
        self.channel_refresh_request_ids.remove(channel);
    }

    pub(crate) fn next_private_channel_candidates_request_id(&mut self) -> u64 {
        self.private_channel_candidates_request_id =
            self.private_channel_candidates_request_id.saturating_add(1);
        self.private_channel_candidates_request_id
    }

    pub(crate) fn invalidate_private_channel_candidates_request(&mut self) {
        self.private_channel_candidates_request_id =
            self.private_channel_candidates_request_id.saturating_add(1);
        self.private_channel_candidates_loading = false;
    }

    pub(crate) fn next_fast_auth_request_id(&mut self) -> u64 {
        self.fast_auth_request_id = self.fast_auth_request_id.saturating_add(1);
        self.fast_auth_request_id
    }

    pub(crate) fn invalidate_fast_auth_request(&mut self) {
        self.fast_auth_request_id = self.fast_auth_request_id.saturating_add(1);
        self.fast_auth_in_flight = false;
    }

    // `posts` is kept sorted newest-first and truncated to the render limit by
    // the feed update path, so views can borrow it directly.
    pub(crate) fn visible_posts(&self) -> &[TelegramFeedPost] {
        &self.posts
    }

    pub(crate) fn rebuild_ticker_mention_resolver(&mut self, symbols: &[ExchangeSymbol]) {
        self.ticker_mention_resolver = SymbolMentionResolver::from_symbols(symbols);
    }

    pub(crate) fn resolve_ticker_mentions(&self, text: &str) -> Vec<SymbolMention> {
        self.ticker_mention_resolver.resolve(text)
    }

    pub(crate) fn has_seen_posts_for_channel(&self, channel: &str) -> bool {
        self.seen_post_ids
            .get(channel)
            .is_some_and(|ids| !ids.is_empty())
            || self.posts.iter().any(|post| post.channel == channel)
    }

    pub(crate) fn record_seen_post(&mut self, channel: &str, message_id: u64) -> bool {
        let ids = self.seen_post_ids.entry(channel.to_string()).or_default();
        if ids.contains(&message_id) {
            return true;
        }

        ids.push_back(message_id);
        while ids.len() > TELEGRAM_FEED_SEEN_ID_LIMIT_PER_CHANNEL {
            let _ = ids.pop_front();
        }
        false
    }

    pub(crate) fn clear_seen_posts_for_channel(&mut self, channel: &str) {
        self.seen_post_ids.remove(channel);
    }

    pub(crate) fn selected_channel_count(&self) -> usize {
        self.channels.len() + self.private_channels.len()
    }

    pub(crate) fn private_channel_selected(&self, peer_id: i64) -> bool {
        self.private_channels
            .iter()
            .any(|channel| channel.peer_id == peer_id)
    }

    pub(crate) fn feed_source_selected(&self, source: &str) -> bool {
        self.channels.iter().any(|channel| channel == source)
            || telegram_private_channel_peer_id_from_key(source)
                .is_some_and(|peer_id| self.private_channel_selected(peer_id))
    }

    pub(crate) fn available_private_channel_candidates(
        &self,
    ) -> Vec<&TelegramPrivateChannelCandidate> {
        self.private_channel_candidates
            .iter()
            .filter(|candidate| !self.private_channel_selected(candidate.peer_id))
            .collect()
    }

    pub(crate) fn record_fast_connection_event(&mut self, now_ms: u64) {
        self.fast_last_event_ms = Some(now_ms);
    }

    pub(crate) fn clear_fast_connection_event(&mut self) {
        self.fast_last_event_ms = None;
    }

    pub(crate) fn fast_connection_stale(&self, now_ms: u64) -> bool {
        self.fast_last_event_ms
            .map(|last_event_ms| {
                now_ms.saturating_sub(last_event_ms) > TELEGRAM_FAST_STALE_AFTER_MS
            })
            .unwrap_or(true)
    }

    /// True once a Fast Mode session is established (either confirmed by the live
    /// stream or by a completed sign-in).
    pub(crate) fn signed_in(&self) -> bool {
        self.fast_connected || matches!(self.fast_auth_stage, TelegramFastAuthStage::SignedIn)
    }

    /// Which of the four pane render states is active. The pane is one small state
    /// machine: a connected (or public-mode) feed, the two sign-in steps, or the
    /// first-run onboarding screen.
    pub(crate) fn current_screen(&self) -> TelegramFeedScreen {
        if self.signed_in() {
            return TelegramFeedScreen::LiveFeed;
        }
        if self.fast_mode_enabled {
            return match self.fast_auth_stage {
                TelegramFastAuthStage::CodeRequested | TelegramFastAuthStage::PasswordRequired => {
                    TelegramFeedScreen::SignInCode
                }
                _ => TelegramFeedScreen::SignInPhone,
            };
        }
        if self.onboarding_dismissed {
            TelegramFeedScreen::LiveFeed
        } else {
            TelegramFeedScreen::Connect
        }
    }
}

/// The four render states of the Telegram Feed pane. See
/// [`TelegramFeedState::current_screen`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TelegramFeedScreen {
    Connect,
    SignInPhone,
    SignInCode,
    LiveFeed,
}

pub(crate) fn default_telegram_feed_channels() -> Vec<String> {
    vec!["marketfeed".to_string()]
}

pub(crate) fn default_telegram_country_code() -> String {
    "+1".to_string()
}

/// Curated dialing codes offered by the sign-in country-code picker. Not
/// exhaustive — users who need another code can paste a full `+…` number into
/// the phone field, which is respected verbatim.
pub(crate) const TELEGRAM_COUNTRY_CODES: &[&str] = &[
    "+1", "+44", "+7", "+33", "+49", "+34", "+39", "+31", "+41", "+46", "+47", "+351", "+61",
    "+64", "+81", "+82", "+86", "+852", "+886", "+91", "+92", "+62", "+63", "+65", "+66", "+84",
    "+971", "+972", "+966", "+90", "+20", "+27", "+234", "+254", "+55", "+52", "+54", "+57", "+56",
    "+380", "+48", "+420", "+30", "+353", "+358",
];

/// Build the E.164-style phone number sent to Telegram from the picked dialing
/// code and the national-number field. A value the user typed with its own `+`
/// prefix is treated as already-complete and the picker is ignored.
pub(crate) fn combine_telegram_phone(country_code: &str, national: &str) -> String {
    let national_trimmed = national.trim();
    if national_trimmed.starts_with('+') {
        let digits: String = national_trimmed
            .chars()
            .filter(char::is_ascii_digit)
            .collect();
        return format!("+{digits}");
    }
    let code_digits: String = country_code.chars().filter(char::is_ascii_digit).collect();
    let national_digits: String = national_trimmed
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    format!("+{code_digits}{national_digits}")
}

/// Mask a phone number for display on the code screen, e.g. `+1 415 ••• 2207`.
pub(crate) fn masked_telegram_phone(full: &str) -> String {
    let digits: String = full.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 4 {
        return "your number".to_string();
    }
    let last4 = &digits[digits.len() - 4..];
    format!("•••• {last4}")
}

pub(crate) fn normalized_channel_list(channels: &[String]) -> Vec<String> {
    normalized_channel_list_with_status(channels).0
}

fn normalized_channel_list_with_status(channels: &[String]) -> (Vec<String>, bool) {
    let mut normalized = Vec::new();
    for channel in channels {
        if let Ok(channel) = normalize_public_channel_input(channel)
            && !normalized.contains(&channel)
        {
            if normalized.len() >= TELEGRAM_FEED_MAX_PUBLIC_CHANNELS {
                return (normalized, true);
            }
            normalized.push(channel);
        }
    }
    (normalized, false)
}

pub(crate) fn normalized_private_channel_list(
    channels: &[TelegramFeedPrivateChannelConfig],
) -> Vec<TelegramFeedPrivateChannelConfig> {
    let mut normalized: Vec<TelegramFeedPrivateChannelConfig> = Vec::new();
    for channel in channels {
        if let Some(channel) = channel.normalized()
            && !normalized
                .iter()
                .any(|existing| existing.peer_id == channel.peer_id)
        {
            normalized.push(channel);
        }
    }
    normalized
}

pub(crate) fn normalize_private_channel_title(title: &str, peer_id: i64) -> String {
    let title = normalize_telegram_plain_text(title).trim().to_string();
    if title.is_empty() {
        format!("Private channel {peer_id}")
    } else {
        title
    }
}

pub(crate) fn telegram_private_channel_key(peer_id: i64) -> String {
    format!("{TELEGRAM_PRIVATE_CHANNEL_KEY_PREFIX}{peer_id}")
}

pub(crate) fn telegram_private_channel_peer_id_from_key(key: &str) -> Option<i64> {
    key.strip_prefix(TELEGRAM_PRIVATE_CHANNEL_KEY_PREFIX)
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|peer_id| *peer_id > 0)
}

pub(crate) fn normalize_public_channel_input(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a public Telegram channel".to_string());
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let without_host = without_scheme
        .strip_prefix("t.me/")
        .or_else(|| without_scheme.strip_prefix("telegram.me/"))
        .unwrap_or(without_scheme);
    let without_public_prefix = without_host.strip_prefix("s/").unwrap_or(without_host);
    let channel = without_public_prefix
        .trim_start_matches('@')
        .split(['?', '#', '/'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    if channel.starts_with('+') || channel == "joinchat" || channel == "c" {
        return Err("Only public @username channels are supported".to_string());
    }

    let mut chars = channel.chars();
    let Some(first) = chars.next() else {
        return Err("Enter a public Telegram channel".to_string());
    };
    if !first.is_ascii_alphabetic() {
        return Err("Telegram channel usernames must start with a letter".to_string());
    }
    if !(5..=32).contains(&channel.len()) {
        return Err("Telegram channel usernames must be 5-32 characters".to_string());
    }
    if !channel
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err("Telegram channel usernames can only use letters, numbers, and _".to_string());
    }

    Ok(channel)
}

pub(crate) fn telegram_channel_profile_from_title(
    channel: &str,
    title: Option<&str>,
) -> TelegramChannelProfile {
    let channel = normalize_public_channel_input(channel).unwrap_or_else(|_| channel.to_string());
    let title = title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("@{channel}"));

    TelegramChannelProfile {
        initials: fallback_initials(&title, &channel),
        channel,
        title,
        avatar_url: None,
        avatar_handle: None,
        avatar_loading_url: None,
        avatar_request_id: 0,
        avatar_failed_at_ms: None,
    }
}

pub(crate) fn is_supported_raster_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"\x89PNG\r\n\x1A\n")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(b"BM")
}

pub(crate) fn normalize_telegram_plain_text(input: &str) -> String {
    strip_unsupported_telegram_emoji(input)
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_unsupported_telegram_emoji(input: &str) -> String {
    input
        .chars()
        .filter(|ch| !is_emoji_or_emoji_joiner(*ch))
        .collect()
}

fn is_emoji_or_emoji_joiner(ch: char) -> bool {
    matches!(
        ch as u32,
        0x200D
            | 0x20E3
            | 0xFE00..=0xFE0F
            | 0x2300..=0x23FF
            | 0x2600..=0x27BF
            | 0x2B00..=0x2BFF
            | 0x1F000..=0x1FAFF
            | 0xE0020..=0xE007F
    )
}

pub(crate) fn telegram_age_countdown_label(sent_at_ms: u64, now_ms: u64) -> String {
    format!(
        "{} ago",
        telegram_countdown_duration_label(now_ms.saturating_sub(sent_at_ms))
    )
}

pub(crate) fn telegram_new_message_heat(first_seen_ms: u64, now_ms: u64) -> f32 {
    cooldown_heat(first_seen_ms, now_ms, TELEGRAM_NEW_MESSAGE_COOLDOWN_MS)
}

pub(crate) fn telegram_arrival_latency_label(post: &TelegramFeedPost) -> Option<String> {
    let observed_at_ms = if post.received_at_ms == 0 {
        post.fetched_at_ms
    } else {
        post.received_at_ms
    };
    format_seen_latency_label(post.timestamp_ms, observed_at_ms, post.first_seen_ms)
}

pub(crate) fn telegram_price_impact_pct(
    reference_price: Option<f64>,
    current_price: Option<f64>,
) -> Option<f64> {
    positive_percent_change(current_price, reference_price)
}

fn telegram_countdown_duration_label(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        format!("{duration_ms} ms")
    } else if duration_ms < 60_000 {
        format!("{}.{:03} s", duration_ms / 1_000, duration_ms % 1_000)
    } else if duration_ms < 3_600_000 {
        format!(
            "{}m {:02}s",
            duration_ms / 60_000,
            (duration_ms % 60_000) / 1_000
        )
    } else {
        format!(
            "{}h {:02}m",
            duration_ms / 3_600_000,
            (duration_ms % 3_600_000) / 60_000
        )
    }
}

#[cfg(test)]
mod tests;
