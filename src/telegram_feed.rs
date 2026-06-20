use crate::api::{CLIENT, ExchangeSymbol};
use crate::app_state::{SensitiveString, sensitive_string};
use crate::app_time::{cooldown_heat, now_ms};
use crate::helpers::{
    fallback_initials, format_seen_latency_label, positive_percent_change,
    redact_sensitive_response_text, text_excerpt,
};
use crate::symbol_mentions::{SymbolAliasSource, SymbolMention, SymbolMentionResolver};
use chrono::{DateTime, Utc};
use iced::widget::image::Handle as ImageHandle;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::time::Duration;

const TELEGRAM_WEB_BASE: &str = "https://t.me/s/";
const TELEGRAM_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; Kerosene Telegram Feed; +https://github.com)";
const TELEGRAM_FEED_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const TELEGRAM_FEED_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const TELEGRAM_AVATAR_MAX_BODY_BYTES: usize = 512 * 1024;
const TELEGRAM_MEDIA_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
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
    ) -> Vec<TelegramPrivateChannelCandidate> {
        self.private_channel_candidates
            .iter()
            .filter(|candidate| !self.private_channel_selected(candidate.peer_id))
            .cloned()
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

pub(crate) async fn fetch_telegram_channel_posts(
    channel: String,
) -> Result<TelegramFeedPage, String> {
    let channel = normalize_public_channel_input(&channel)?;
    let url = format!("{TELEGRAM_WEB_BASE}{channel}");
    let request_started_ms = now_ms();
    let response = CLIENT
        .get(&url)
        .header(USER_AGENT, TELEGRAM_USER_AGENT)
        .timeout(TELEGRAM_FEED_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("@{channel} request failed: {e}"))?;
    let status = response.status();
    let body_bytes = read_response_body_limited(
        response,
        TELEGRAM_FEED_MAX_BODY_BYTES,
        &format!("@{channel}"),
    )
    .await?;
    let fetched_at_ms = now_ms();
    let request_duration_ms = fetched_at_ms.saturating_sub(request_started_ms);
    let body = String::from_utf8_lossy(&body_bytes);

    if !status.is_success() {
        let preview = text_excerpt(&body, 160);
        return if preview.is_empty() {
            Err(format!("@{channel} request failed with HTTP {status}"))
        } else {
            Err(format!(
                "@{channel} request failed with HTTP {status}: {preview}"
            ))
        };
    }

    let profile = parse_telegram_channel_profile(&channel, &body);
    let posts = parse_telegram_channel_html(&channel, &body, TELEGRAM_FEED_FETCH_LIMIT)
        .into_iter()
        .map(|post| post.with_fetch_timing(request_started_ms, fetched_at_ms, request_duration_ms))
        .collect::<Vec<_>>();
    if posts.is_empty() {
        Err(format!(
            "@{channel} returned no public posts. Check that it is a public channel."
        ))
    } else {
        Ok(TelegramFeedPage { profile, posts })
    }
}

pub(crate) async fn fetch_telegram_avatar_bytes(
    channel: String,
    avatar_url: String,
) -> Result<Vec<u8>, String> {
    let channel = normalize_public_channel_input(&channel)?;
    let response = CLIENT
        .get(&avatar_url)
        .header(USER_AGENT, TELEGRAM_USER_AGENT)
        .timeout(TELEGRAM_FEED_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("@{channel} avatar request failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "@{channel} avatar request failed with HTTP {status}"
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let body = read_response_body_limited(
        response,
        TELEGRAM_AVATAR_MAX_BODY_BYTES,
        &format!("@{channel} avatar"),
    )
    .await?;
    if !is_supported_raster_image(&body) {
        let content_type = content_type.unwrap_or_else(|| "unknown content type".to_string());
        return Err(format!(
            "@{channel} avatar response was not a supported image: {content_type}"
        ));
    }

    Ok(body)
}

pub(crate) async fn fetch_telegram_media_bytes(
    channel: String,
    message_id: u64,
    media_url: String,
) -> Result<Vec<u8>, String> {
    let channel = normalize_public_channel_input(&channel)?;
    let response = CLIENT
        .get(&media_url)
        .header(USER_AGENT, TELEGRAM_USER_AGENT)
        .timeout(TELEGRAM_FEED_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("@{channel}/{message_id} media request failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "@{channel}/{message_id} media request failed with HTTP {status}"
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let body = read_response_body_limited(
        response,
        TELEGRAM_MEDIA_MAX_BODY_BYTES,
        &format!("@{channel}/{message_id} media"),
    )
    .await?;
    if !is_supported_raster_image(&body) {
        let content_type = content_type.unwrap_or_else(|| "unknown content type".to_string());
        return Err(format!(
            "@{channel}/{message_id} media response was not a supported image: {content_type}"
        ));
    }

    Ok(body)
}

async fn read_response_body_limited(
    mut response: reqwest::Response,
    max_body_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|len| len > max_body_bytes as u64)
    {
        return Err(format!(
            "{label} response was too large: more than {max_body_bytes} bytes"
        ));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("{label} response read failed: {e}"))?
    {
        if body.len() + chunk.len() > max_body_bytes {
            return Err(format!(
                "{label} response was too large: more than {max_body_bytes} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub(crate) fn parse_telegram_channel_profile(channel: &str, html: &str) -> TelegramChannelProfile {
    let channel = normalize_public_channel_input(channel).unwrap_or_else(|_| channel.to_string());
    let title = html_between(html, "tgme_channel_info_header_title", "</div>")
        .map(|html| html_to_plain_text(&html))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| format!("@{channel}"));
    let photo_block = html
        .find("tgme_page_photo_image")
        .map(|start| {
            let end = html[start..]
                .find("</i>")
                .map(|offset| start + offset + 4)
                .unwrap_or_else(|| html.len().min(start + 2_000));
            &html[start..end]
        })
        .unwrap_or_default();
    let avatar_url =
        attr_value(photo_block, "src=\"").and_then(|url| normalize_telegram_asset_url(&url));
    let initials = attr_value(photo_block, "data-content=\"")
        .map(|value| html_to_plain_text(&value))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback_initials(&title, &channel));

    TelegramChannelProfile {
        channel,
        title,
        initials,
        avatar_url,
        avatar_handle: None,
        avatar_loading_url: None,
        avatar_request_id: 0,
        avatar_failed_at_ms: None,
    }
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

pub(crate) fn parse_telegram_channel_html(
    channel: &str,
    html: &str,
    limit: usize,
) -> Vec<TelegramFeedPost> {
    let channel = normalize_public_channel_input(channel).unwrap_or_else(|_| channel.to_string());
    let mut posts = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = html[cursor..].find("data-post=\"") {
        let post_start = cursor + relative_start;
        let block_end = html[post_start + 11..]
            .find("data-post=\"")
            .map(|offset| post_start + 11 + offset)
            .unwrap_or(html.len());
        let block = &html[post_start..block_end];
        if let Some(post) = parse_telegram_message_block(&channel, block) {
            posts.push(post);
        }
        cursor = block_end;
    }

    posts.sort_by(|left, right| {
        left.timestamp_ms
            .cmp(&right.timestamp_ms)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    if posts.len() > limit {
        posts = posts.split_off(posts.len() - limit);
    }
    posts.reverse();
    posts
}

fn parse_telegram_message_block(channel: &str, block: &str) -> Option<TelegramFeedPost> {
    let data_post = attr_value(block, "data-post=\"")?;
    let (_, id) = data_post.rsplit_once('/')?;
    let message_id = id.parse::<u64>().ok()?;
    let datetime = attr_value(block, "datetime=\"")?;
    let timestamp_ms = DateTime::parse_from_rfc3339(&datetime)
        .ok()?
        .with_timezone(&Utc)
        .timestamp_millis();
    let timestamp_ms = u64::try_from(timestamp_ms).ok()?;
    let media = parse_telegram_post_media(block);
    let caption = html_between(block, "tgme_widget_message_text js-message_text", "</div>")
        .map(|text_html| html_to_plain_text(&text_html))
        .filter(|caption| !caption.trim().is_empty());
    // A media-only post renders its preview instead of a placeholder string; only
    // posts with neither a caption nor a displayable preview fall back to a label,
    // and a post with nothing to show at all is dropped.
    let text = match caption {
        Some(caption) => caption,
        None if media.is_some() => String::new(),
        None => telegram_message_fallback_text(block),
    };
    if text.trim().is_empty() && media.is_none() {
        return None;
    }

    Some(TelegramFeedPost {
        channel: channel.to_string(),
        message_id,
        text,
        timestamp_ms,
        source: TelegramFeedPostSource::PublicPoll,
        received_at_ms: 0,
        applied_at_ms: 0,
        fetched_at_ms: 0,
        request_started_ms: 0,
        request_duration_ms: 0,
        first_seen_ms: 0,
        url: format!("https://t.me/{channel}/{message_id}"),
        ticker_mentions: Vec::new(),
        media,
    })
}

fn attr_value(block: &str, marker: &str) -> Option<String> {
    let start = block.find(marker)? + marker.len();
    let end = block[start..].find('"')?;
    Some(block[start..start + end].to_string())
}

fn html_between(block: &str, class_marker: &str, end_marker: &str) -> Option<String> {
    let class_start = block.find(class_marker)?;
    let content_start = block[class_start..].find('>')? + class_start + 1;
    let content_end = block[content_start..].find(end_marker)? + content_start;
    Some(block[content_start..content_end].to_string())
}

fn html_to_plain_text(html: &str) -> String {
    let normalized = html
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    let mut out = String::with_capacity(normalized.len());
    let mut in_tag = false;
    for ch in normalized.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    let plain_text = decode_html_entities(&out);
    normalize_telegram_plain_text(&plain_text)
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

fn normalize_telegram_asset_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        None
    } else if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        Some(trimmed.to_string())
    } else if trimmed.starts_with("//") {
        Some(format!("https:{trimmed}"))
    } else if trimmed.starts_with('/') {
        Some(format!("https://t.me{trimmed}"))
    } else {
        None
    }
}

/// Extracts the first displayable preview image from a public message block,
/// classifying it so the card can label media that has not loaded yet. Returns
/// `None` when the block has no extractable preview URL (the caller then keeps
/// the textual `[photo]`/`[video]` fallback instead).
fn parse_telegram_post_media(block: &str) -> Option<TelegramPostMedia> {
    // Stickers expose a static WebP preview via data-webp on t.me/s.
    if block.contains("tgme_widget_message_sticker")
        && let Some(url) = attr_value(block, "data-webp=\"")
            .and_then(|url| normalize_telegram_asset_url(&decode_html_entities(url.trim())))
            .or_else(|| telegram_media_background_url(block, "tgme_widget_message_sticker"))
    {
        return Some(TelegramPostMedia::from_url(TelegramMediaKind::Sticker, url));
    }
    // Round video messages carry their preview frame as a background image.
    if let Some(url) = telegram_media_background_url(block, "tgme_widget_message_roundvideo_thumb")
    {
        return Some(TelegramPostMedia::from_url(TelegramMediaKind::Video, url));
    }
    // Videos and GIFs both surface a still preview frame; t.me badges GIFs with a
    // "GIF" duration label (`<time ...>GIF</time>`). Matching the closing `</time>`
    // keeps a caption that merely contains the word "gif" from being misclassified.
    if let Some(url) = telegram_media_background_url(block, "tgme_widget_message_video_thumb") {
        let kind = if block.to_ascii_lowercase().contains(">gif</time>") {
            TelegramMediaKind::Gif
        } else {
            TelegramMediaKind::Video
        };
        return Some(TelegramPostMedia::from_url(kind, url));
    }
    // Plain photos.
    if let Some(url) = telegram_media_background_url(block, "tgme_widget_message_photo_wrap") {
        return Some(TelegramPostMedia::from_url(TelegramMediaKind::Photo, url));
    }
    None
}

/// Reads a `background-image:url(...)` value from the opening tag that carries
/// `class_marker`. Scanning only that tag keeps an element without an inline
/// style from borrowing a sibling element's preview URL.
fn telegram_media_background_url(block: &str, class_marker: &str) -> Option<String> {
    let class_start = block.find(class_marker)?;
    let tag_end = block[class_start..]
        .find('>')
        .map(|offset| class_start + offset)
        .unwrap_or(block.len());
    let tag = &block[class_start..tag_end];
    let background = tag.find("background-image")?;
    let url_open = tag[background..].find("url(")? + background + 4;
    // Decode entities before stripping quotes and locating the closing delimiter,
    // so an entity-encoded inner quote (`url(&#39;...&#39;)`) is handled like a
    // literal one instead of leaking into the extracted URL.
    let decoded = decode_html_entities(&tag[url_open..]);
    let value = decoded.trim_start_matches(['\'', '"', ' ']);
    let end = value.find([')', '\'', '"'])?;
    normalize_telegram_asset_url(value[..end].trim())
}

fn telegram_message_fallback_text(block: &str) -> String {
    if block.contains("tgme_widget_message_photo") {
        "[photo]".to_string()
    } else if block.contains("tgme_widget_message_video") {
        "[video]".to_string()
    } else if block.contains("tgme_widget_message_document") {
        "[file]".to_string()
    } else if block.contains("tgme_widget_message_poll") {
        "[poll]".to_string()
    } else if block.contains("tgme_widget_message_voice") {
        "[voice]".to_string()
    } else if block.contains("tgme_widget_message_roundvideo") {
        "[video message]".to_string()
    } else {
        String::new()
    }
}

fn decode_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find(';') {
            let entity = &rest[1..end];
            if let Some(decoded) = decode_entity(entity) {
                out.push(decoded);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        entity if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        entity if entity.starts_with('#') => {
            entity[1..].parse::<u32>().ok().and_then(char::from_u32)
        }
        _ => None,
    }
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
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"
<div class="tgme_channel_info">
  <div class="tgme_channel_info_header">
    <i class="tgme_page_photo_image bgcolor3" data-content="MF"><img src="https://cdn4.telesco.pe/file/avatar.jpg"></i>
    <div class="tgme_channel_info_header_title"><span dir="auto">Market News Feed</span></div>
    <div class="tgme_channel_info_header_username"><a href="https://t.me/marketfeed">@marketfeed</a></div>
  </div>
</div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/10">
<div class="tgme_widget_message_text js-message_text" dir="auto">FIRST &amp; <b>FAST</b><br/>line two <a href="https://example.com">link</a></div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/10"><time datetime="2026-05-31T17:50:14+00:00" class="time">17:50</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/11">
<div class="tgme_widget_message_text js-message_text" dir="auto">SECOND &#39;POST&#39;</div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/11"><time datetime="2026-05-31T18:00:00+00:00" class="time">18:00</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/12">
<a class="tgme_widget_message_photo_wrap" href="https://t.me/marketfeed/12"><i class="tgme_widget_message_photo"></i></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/12"><time datetime="2026-05-31T18:01:00+00:00" class="time">18:01</time></a>
</div></div>
"#;

    #[test]
    fn telegram_feed_state_debug_redacts_fast_credentials() {
        let mut state = TelegramFeedState::new(&[], &[], false, false, Some(12345), true, false);
        state.fast_api_hash_input = "hash-secret".to_string().into();
        state.fast_phone_input = "+15555550123".to_string();
        state.fast_code_input = "code-secret".to_string().into();
        state.fast_password_input = "password-secret".to_string().into();
        state.fast_password_hint = Some("hint-secret".to_string());
        state.fast_status = Some((
            "api_hash=hash-secret phone_code=code-secret hint-secret".to_string(),
            true,
        ));

        let rendered = format!("{state:?}");

        assert!(rendered.contains("<redacted>"));
        for secret in [
            "12345",
            "hash-secret",
            "+15555550123",
            "code-secret",
            "password-secret",
            "hint-secret",
        ] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn telegram_fast_input_fields_debug_redact_when_formatted_directly() {
        let mut state = TelegramFeedState::new(&[], &[], false, false, Some(12345), true, false);
        state.fast_api_hash_input = "hash-secret".to_string().into();
        state.fast_code_input = "code-secret".to_string().into();
        state.fast_password_input = "password-secret".to_string().into();

        let rendered = format!(
            "{:?} {:?} {:?}",
            state.fast_api_hash_input, state.fast_code_input, state.fast_password_input
        );

        assert!(rendered.contains("<redacted>"));
        for secret in ["hash-secret", "code-secret", "password-secret"] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn telegram_fast_auth_outcome_debug_redacts_password_hint() {
        let outcome = TelegramFastAuthOutcome::PasswordRequired {
            hint: Some("hint-secret".to_string()),
        };

        let rendered = format!("{outcome:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("hint-secret"));
    }

    #[test]
    fn telegram_fast_feed_event_debug_redacts_status_and_errors() {
        let status = TelegramFastFeedEvent::Status {
            connected: false,
            auth_required: true,
            message: "api_hash=hash-secret phone_code=code-secret".to_string(),
        };
        let loaded_error = TelegramFastFeedEvent::Loaded(
            "private:42".to_string(),
            Box::new(Err(
                "phone_code_hash=hash-secret password=password-secret".to_string()
            )),
        );

        let rendered = format!("{status:?} {loaded_error:?}");

        assert!(rendered.contains("<redacted>"));
        for secret in ["hash-secret", "code-secret", "password-secret"] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
        assert!(!rendered.contains("private:42"));
    }

    #[test]
    fn telegram_fast_feed_event_debug_summarizes_loaded_pages() {
        let page = TelegramFeedPage {
            profile: TelegramChannelProfile {
                channel: "private:42".to_string(),
                title: "Private Feed".to_string(),
                initials: "PF".to_string(),
                avatar_url: None,
                avatar_handle: None,
                avatar_loading_url: None,
                avatar_request_id: 0,
                avatar_failed_at_ms: None,
            },
            posts: vec![TelegramFeedPost {
                channel: "private:42".to_string(),
                message_id: 1,
                text: "post body should not appear in debug".to_string(),
                timestamp_ms: 1,
                source: TelegramFeedPostSource::FastLive,
                received_at_ms: 2,
                applied_at_ms: 2,
                fetched_at_ms: 2,
                request_started_ms: 1,
                request_duration_ms: 1,
                first_seen_ms: 2,
                url: "https://t.me/s/private/1".to_string(),
                ticker_mentions: Vec::new(),
                media: None,
            }],
        };

        let rendered = format!(
            "{:?}",
            TelegramFastFeedEvent::Loaded("private:42".to_string(), Box::new(Ok(page)))
        );

        assert!(rendered.contains("posts: 1"));
        assert!(rendered.contains("<private>"));
        assert!(!rendered.contains("private:42"));
        assert!(!rendered.contains("Private Feed"));
        assert!(!rendered.contains("post body should not appear in debug"));
        assert!(!rendered.contains("https://t.me/s/private/1"));
    }

    #[test]
    fn telegram_private_channel_debug_redacts_identity() {
        let config = TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Alpha".to_string(),
        };
        let candidate = TelegramPrivateChannelCandidate {
            peer_id: 43,
            title: "Private Beta".to_string(),
            avatar_handle: None,
        };

        let rendered = format!("{config:?} {candidate:?}");

        assert!(rendered.contains("<redacted>"));
        for secret in ["42", "43", "Private Alpha", "Private Beta"] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn telegram_feed_post_debug_redacts_body_and_private_source() {
        let post = TelegramFeedPost {
            channel: "private:42".to_string(),
            message_id: 7,
            text: "private post text".to_string(),
            timestamp_ms: 1,
            source: TelegramFeedPostSource::FastLive,
            received_at_ms: 2,
            applied_at_ms: 2,
            fetched_at_ms: 2,
            request_started_ms: 1,
            request_duration_ms: 1,
            first_seen_ms: 2,
            url: "https://t.me/c/42/7".to_string(),
            ticker_mentions: vec![TelegramTickerMention {
                symbol: "BTC".to_string(),
                ticker: "BTC".to_string(),
                matched_text: "private post text".to_string(),
                source: SymbolAliasSource::Ticker,
                confidence: 100,
                reference_price: None,
                reference_seen_ms: 0,
            }],
            media: None,
        };

        let rendered = format!("{post:?}");

        assert!(rendered.contains("<private>"));
        assert!(rendered.contains("ticker_mentions: 1"));
        for secret in ["private:42", "private post text", "https://t.me/c/42/7"] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn telegram_feed_state_debug_redacts_private_feed_content() {
        let private_channels = vec![TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Alpha".to_string(),
        }];
        let mut state = TelegramFeedState::new(
            &[],
            &private_channels,
            false,
            true,
            Some(12345),
            true,
            false,
        );
        state.private_channel_candidates = vec![TelegramPrivateChannelCandidate {
            peer_id: 43,
            title: "Private Beta".to_string(),
            avatar_handle: None,
        }];
        state.channel_input = "private:42".to_string();
        state.channel_profiles.insert(
            "private:42".to_string(),
            TelegramChannelProfile {
                channel: "private:42".to_string(),
                title: "Private Alpha".to_string(),
                initials: "PA".to_string(),
                avatar_url: Some("https://cdn.telegram.example/private-alpha.jpg".to_string()),
                avatar_handle: None,
                avatar_loading_url: None,
                avatar_request_id: 3,
                avatar_failed_at_ms: None,
            },
        );
        state.posts.push(TelegramFeedPost {
            channel: "private:42".to_string(),
            message_id: 7,
            text: "private post text".to_string(),
            timestamp_ms: 1,
            source: TelegramFeedPostSource::FastLive,
            received_at_ms: 2,
            applied_at_ms: 2,
            fetched_at_ms: 2,
            request_started_ms: 1,
            request_duration_ms: 1,
            first_seen_ms: 2,
            url: "https://t.me/c/42/7".to_string(),
            ticker_mentions: Vec::new(),
            media: None,
        });
        state.record_seen_post("private:42", 7);
        state
            .channel_refresh_request_ids
            .insert("private:42".to_string(), 11);
        state.loading_channels.push("private:42".to_string());
        state
            .background_loading_channels
            .push("private:42".to_string());

        let rendered = format!("{state:?}");

        assert!(rendered.contains("<private>"));
        assert!(rendered.contains("private: 1"));
        assert!(rendered.contains("private_channels: 1"));
        for secret in [
            "private:42",
            "Private Alpha",
            "Private Beta",
            "private post text",
            "https://t.me/c/42/7",
            "https://cdn.telegram.example/private-alpha.jpg",
        ] {
            assert!(!rendered.contains(secret), "debug leaked {secret}");
        }
    }

    #[test]
    fn normalizes_public_channel_inputs() {
        assert_eq!(
            normalize_public_channel_input("@MarketFeed").unwrap(),
            "marketfeed"
        );
        assert_eq!(
            normalize_public_channel_input("https://t.me/s/MarketFeed?before=1").unwrap(),
            "marketfeed"
        );
        assert!(normalize_public_channel_input("https://t.me/+private").is_err());
        assert!(normalize_public_channel_input("bad-channel").is_err());
    }

    #[test]
    fn normalized_channel_list_dedupes_and_caps_public_channels() {
        let channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS + 4)
            .map(|index| format!("channel_{index}"))
            .chain(std::iter::once("channel_1".to_string()))
            .collect::<Vec<_>>();

        let normalized = normalized_channel_list(&channels);

        assert_eq!(normalized.len(), TELEGRAM_FEED_MAX_PUBLIC_CHANNELS);
        assert_eq!(normalized[0], "channel_0");
        assert_eq!(normalized[1], "channel_1");
        assert_eq!(
            normalized[TELEGRAM_FEED_MAX_PUBLIC_CHANNELS - 1],
            format!("channel_{}", TELEGRAM_FEED_MAX_PUBLIC_CHANNELS - 1)
        );
        assert!(!normalized.contains(&format!("channel_{TELEGRAM_FEED_MAX_PUBLIC_CHANNELS}")));
    }

    #[test]
    fn saved_public_channel_cap_sets_runtime_warning() {
        let channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS + 1)
            .map(|index| format!("channel_{index}"))
            .collect::<Vec<_>>();

        let state = TelegramFeedState::new(&channels, &[], false, false, None, true, false);

        assert_eq!(state.channels.len(), TELEGRAM_FEED_MAX_PUBLIC_CHANNELS);
        assert_eq!(
            state.last_error,
            Some(format!(
                "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels; extra saved channels were ignored"
            ))
        );
    }

    #[test]
    fn normalized_private_channel_list_dedupes_and_sanitizes_titles() {
        let channels = vec![
            TelegramFeedPrivateChannelConfig {
                peer_id: 42,
                title: "  Macro & News  ".to_string(),
            },
            TelegramFeedPrivateChannelConfig {
                peer_id: 42,
                title: "Duplicate".to_string(),
            },
            TelegramFeedPrivateChannelConfig {
                peer_id: 0,
                title: "Invalid".to_string(),
            },
            TelegramFeedPrivateChannelConfig {
                peer_id: 43,
                title: String::new(),
            },
        ];

        let normalized = normalized_private_channel_list(&channels);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].peer_id, 42);
        assert_eq!(normalized[0].title, "Macro & News");
        assert_eq!(normalized[1].title, "Private channel 43");
        assert_eq!(normalized[0].key(), "private:42");
        assert_eq!(
            telegram_private_channel_peer_id_from_key("private:42"),
            Some(42)
        );
        assert_eq!(
            telegram_private_channel_peer_id_from_key("marketfeed"),
            None
        );
    }

    #[test]
    fn parses_channel_profile_avatar_metadata() {
        let profile = parse_telegram_channel_profile("marketfeed", SAMPLE_HTML);

        assert_eq!(profile.channel, "marketfeed");
        assert_eq!(profile.title, "Market News Feed");
        assert_eq!(profile.initials, "MF");
        assert_eq!(
            profile.avatar_url.as_deref(),
            Some("https://cdn4.telesco.pe/file/avatar.jpg")
        );
    }

    #[test]
    fn parses_public_channel_html_and_limits_latest_posts() {
        let posts = parse_telegram_channel_html("marketfeed", SAMPLE_HTML, 1);

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].message_id, 12);
        assert_eq!(posts[0].text, "[photo]");
        assert_eq!(posts[0].url, "https://t.me/marketfeed/12");
    }

    #[test]
    fn decodes_tags_breaks_and_entities() {
        let posts = parse_telegram_channel_html("marketfeed", SAMPLE_HTML, 10);
        let first = posts.iter().find(|post| post.message_id == 10).unwrap();

        assert_eq!(first.text, "FIRST & FAST\nline two link");
    }

    #[test]
    fn strips_emoji_that_bundled_fonts_do_not_render() {
        let html = r#"
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/20">
<div class="tgme_widget_message_text js-message_text" dir="auto">🚨 BREAKING ⚡️ BTC pumps 🟢<br/>alpha 👨‍💻 desk</div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/20"><time datetime="2026-05-31T18:02:00+00:00" class="time">18:02</time></a>
</div></div>
"#;
        let posts = parse_telegram_channel_html("marketfeed", html, 10);

        assert_eq!(posts[0].text, "BREAKING BTC pumps\nalpha desk");
    }

    #[test]
    fn skips_messages_that_only_contain_stripped_emoji() {
        let html = r#"
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/21">
<div class="tgme_widget_message_text js-message_text" dir="auto">🔥🔥🔥</div>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/21"><time datetime="2026-05-31T18:03:00+00:00" class="time">18:03</time></a>
</div></div>
"#;
        let posts = parse_telegram_channel_html("marketfeed", html, 10);

        assert!(posts.is_empty());
    }

    #[test]
    fn parses_media_only_posts_with_fallback_text() {
        let posts = parse_telegram_channel_html("marketfeed", SAMPLE_HTML, 10);
        let media_post = posts.iter().find(|post| post.message_id == 12).unwrap();

        // No preview URL is extractable from this block, so the post keeps the
        // textual placeholder and carries no displayable media.
        assert_eq!(media_post.text, "[photo]");
        assert!(media_post.media.is_none());
    }

    const MEDIA_HTML: &str = r#"
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/30">
<a class="tgme_widget_message_photo_wrap" style="background-image:url('https://cdn4.telesco.pe/file/photo30.jpg')" href="https://t.me/marketfeed/30"></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/30"><time datetime="2026-05-31T19:00:00+00:00" class="time">19:00</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/31">
<div class="tgme_widget_message_text js-message_text" dir="auto">caption here</div>
<a class="tgme_widget_message_video_player blured" href="https://t.me/marketfeed/31"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn4.telesco.pe/file/video31.jpg')"></i><time class="message_video_duration js-message_video_duration">0:15</time></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/31"><time datetime="2026-05-31T19:01:00+00:00" class="time">19:01</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/32">
<i class="tgme_widget_message_sticker" data-webp="https://cdn4.telesco.pe/file/sticker32.webp" style="width:128px;height:128px"></i>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/32"><time datetime="2026-05-31T19:02:00+00:00" class="time">19:02</time></a>
</div></div>
<div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message js-widget_message" data-post="marketfeed/33">
<a class="tgme_widget_message_video_player" href="https://t.me/marketfeed/33"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn4.telesco.pe/file/gif33.jpg')"></i><time class="message_video_duration">GIF</time></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/33"><time datetime="2026-05-31T19:03:00+00:00" class="time">19:03</time></a>
</div></div>
"#;

    #[test]
    fn parses_attached_media_previews() {
        let posts = parse_telegram_channel_html("marketfeed", MEDIA_HTML, 10);

        let photo = posts.iter().find(|post| post.message_id == 30).unwrap();
        // A captionless photo renders its preview, so it carries no placeholder text.
        assert!(photo.text.is_empty());
        let photo_media = photo.media.as_ref().expect("photo media");
        assert_eq!(photo_media.kind, TelegramMediaKind::Photo);
        assert_eq!(
            photo_media.url.as_deref(),
            Some("https://cdn4.telesco.pe/file/photo30.jpg")
        );
        assert!(photo_media.handle.is_none());

        let video = posts.iter().find(|post| post.message_id == 31).unwrap();
        assert_eq!(video.text, "caption here");
        let video_media = video.media.as_ref().expect("video media");
        assert_eq!(video_media.kind, TelegramMediaKind::Video);
        assert_eq!(
            video_media.url.as_deref(),
            Some("https://cdn4.telesco.pe/file/video31.jpg")
        );

        let sticker = posts.iter().find(|post| post.message_id == 32).unwrap();
        let sticker_media = sticker.media.as_ref().expect("sticker media");
        assert_eq!(sticker_media.kind, TelegramMediaKind::Sticker);
        assert_eq!(
            sticker_media.url.as_deref(),
            Some("https://cdn4.telesco.pe/file/sticker32.webp")
        );

        let gif = posts.iter().find(|post| post.message_id == 33).unwrap();
        assert_eq!(
            gif.media.as_ref().expect("gif media").kind,
            TelegramMediaKind::Gif
        );
    }

    #[test]
    fn extracts_background_image_url_with_entity_encoded_quotes() {
        let block = r#"<div data-post="marketfeed/40">
<a class="tgme_widget_message_photo_wrap" style="background-image:url(&#39;https://cdn4.telesco.pe/file/enc40.jpg&#39;)" href="https://t.me/marketfeed/40"></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/40"><time datetime="2026-05-31T20:00:00+00:00">20:00</time></a>
</div>"#;
        let posts = parse_telegram_channel_html("marketfeed", block, 10);

        let media = posts[0].media.as_ref().expect("photo media");
        assert_eq!(media.kind, TelegramMediaKind::Photo);
        assert_eq!(
            media.url.as_deref(),
            Some("https://cdn4.telesco.pe/file/enc40.jpg")
        );
    }

    #[test]
    fn video_with_gif_caption_text_is_not_classified_as_gif() {
        // The caption merely contains the word "gif"; only the duration badge
        // (`<time>GIF</time>`) should drive GIF classification.
        let block = r#"<div data-post="marketfeed/41">
<div class="tgme_widget_message_text js-message_text" dir="auto">is this a gif or a video</div>
<a class="tgme_widget_message_video_player" href="https://t.me/marketfeed/41"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn4.telesco.pe/file/vid41.jpg')"></i><time class="message_video_duration">0:30</time></a>
<a class="tgme_widget_message_date" href="https://t.me/marketfeed/41"><time datetime="2026-05-31T20:01:00+00:00">20:01</time></a>
</div>"#;
        let posts = parse_telegram_channel_html("marketfeed", block, 10);

        assert_eq!(
            posts[0].media.as_ref().expect("video media").kind,
            TelegramMediaKind::Video
        );
    }

    #[test]
    fn telegram_post_media_debug_redacts_url() {
        let media = TelegramPostMedia::from_url(
            TelegramMediaKind::Photo,
            "https://t.me/c/42/7/private-file.jpg".to_string(),
        );

        let rendered = format!("{media:?}");

        assert!(rendered.contains("<url>"));
        assert!(!rendered.contains("https://t.me/c/42/7/private-file.jpg"));
    }

    #[test]
    fn age_countdown_label_includes_precise_elapsed_time() {
        let now = 10_000_000;

        assert_eq!(telegram_age_countdown_label(now - 750, now), "750 ms ago");
        assert_eq!(
            telegram_age_countdown_label(now - 12_345, now),
            "12.345 s ago"
        );
        assert_eq!(
            telegram_age_countdown_label(now - 83_000, now),
            "1m 23s ago"
        );
    }

    #[test]
    fn arrival_latency_label_uses_fetched_at_minus_sent_at() {
        let post = TelegramFeedPost {
            channel: "marketfeed".to_string(),
            message_id: 1,
            text: "fast".to_string(),
            timestamp_ms: 1_000,
            source: TelegramFeedPostSource::PublicPoll,
            received_at_ms: 1_250,
            applied_at_ms: 1_260,
            fetched_at_ms: 1_250,
            request_started_ms: 1_100,
            request_duration_ms: 150,
            first_seen_ms: 1_250,
            url: "https://t.me/marketfeed/1".to_string(),
            ticker_mentions: Vec::new(),
            media: None,
        };

        assert_eq!(
            telegram_arrival_latency_label(&post).unwrap(),
            "seen +250 ms"
        );
    }

    #[test]
    fn arrival_latency_label_hides_historical_fetches() {
        let post = TelegramFeedPost {
            channel: "marketfeed".to_string(),
            message_id: 1,
            text: "old".to_string(),
            timestamp_ms: 1_000,
            source: TelegramFeedPostSource::PublicPoll,
            received_at_ms: 9_000,
            applied_at_ms: 9_010,
            fetched_at_ms: 9_000,
            request_started_ms: 8_850,
            request_duration_ms: 150,
            first_seen_ms: 0,
            url: "https://t.me/marketfeed/1".to_string(),
            ticker_mentions: Vec::new(),
            media: None,
        };

        assert_eq!(telegram_arrival_latency_label(&post), None);
    }

    #[test]
    fn price_impact_pct_uses_reference_price() {
        let impact = telegram_price_impact_pct(Some(100.0), Some(101.5)).unwrap();
        assert!((impact - 1.5).abs() < 1e-9);
        assert_eq!(telegram_price_impact_pct(Some(0.0), Some(101.5)), None);
        assert_eq!(telegram_price_impact_pct(Some(100.0), None), None);
    }

    #[test]
    fn new_message_heat_cools_down_over_time() {
        assert_eq!(telegram_new_message_heat(0, 10_000), 0.0);
        assert_eq!(telegram_new_message_heat(10_000, 10_000), 1.0);
        assert!((telegram_new_message_heat(10_000, 70_000) - 0.5).abs() < f32::EPSILON);
        assert_eq!(
            telegram_new_message_heat(10_000, 10_000 + TELEGRAM_NEW_MESSAGE_COOLDOWN_MS),
            0.0
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_marketfeed_fetches_posts() {
        let page = fetch_telegram_channel_posts("@marketfeed".to_string())
            .await
            .expect("@marketfeed should fetch");
        let posts = page.posts;

        assert!(!posts.is_empty());
        assert!(posts.len() <= TELEGRAM_FEED_FETCH_LIMIT);
        assert!(posts.iter().all(|post| post.channel == "marketfeed"));
        assert!(posts.iter().all(|post| post.fetched_at_ms > 0));
        assert!(posts.iter().all(|post| post.request_started_ms > 0));
        assert_eq!(page.profile.channel, "marketfeed");
    }

    #[tokio::test]
    #[ignore]
    async fn live_marketfeed_fetches_avatar() {
        let page = fetch_telegram_channel_posts("@marketfeed".to_string())
            .await
            .expect("@marketfeed should fetch");
        let avatar_url = page
            .profile
            .avatar_url
            .expect("@marketfeed should expose an avatar URL");
        let avatar = fetch_telegram_avatar_bytes("@marketfeed".to_string(), avatar_url)
            .await
            .expect("@marketfeed avatar should fetch");

        assert!(!avatar.is_empty());
        assert!(avatar.len() <= TELEGRAM_AVATAR_MAX_BODY_BYTES);
    }

    #[test]
    fn current_screen_walks_the_onboarding_state_machine() {
        // First run with no login and onboarding not yet dismissed.
        let mut state = TelegramFeedState::new(&[], &[], false, false, None, true, false);
        assert_eq!(state.current_screen(), TelegramFeedScreen::Connect);

        // Choosing public mode reaches the feed without a login.
        state.onboarding_dismissed = true;
        assert_eq!(state.current_screen(), TelegramFeedScreen::LiveFeed);

        // Entering the connect flow shows the phone step, then the code step.
        state.onboarding_dismissed = false;
        state.fast_mode_enabled = true;
        assert_eq!(state.current_screen(), TelegramFeedScreen::SignInPhone);
        state.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        assert_eq!(state.current_screen(), TelegramFeedScreen::SignInCode);
        state.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
        assert_eq!(state.current_screen(), TelegramFeedScreen::SignInCode);

        // A live session always lands on the feed.
        state.fast_connected = true;
        assert_eq!(state.current_screen(), TelegramFeedScreen::LiveFeed);
        assert!(state.signed_in());
    }

    #[test]
    fn combine_telegram_phone_prefixes_dialing_code_and_keeps_explicit_numbers() {
        assert_eq!(combine_telegram_phone("+1", "415 813 2207"), "+14158132207");
        assert_eq!(
            combine_telegram_phone("+44", "20 7946 0958"),
            "+442079460958"
        );
        // A number the user typed with its own country code is respected verbatim.
        assert_eq!(
            combine_telegram_phone("+1", "+44 20 7946 0958"),
            "+442079460958"
        );
    }

    #[test]
    fn masked_telegram_phone_reveals_only_the_last_four_digits() {
        let masked = masked_telegram_phone("+1 415 813 2207");
        assert!(masked.ends_with("2207"), "got {masked}");
        assert!(masked.contains('•'));
        assert!(!masked.contains("813"));
        assert_eq!(masked_telegram_phone("12"), "your number");
    }
}
