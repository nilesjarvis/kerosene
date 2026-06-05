use crate::api::{CLIENT, ExchangeSymbol};
use crate::helpers::text_excerpt;
use crate::symbol_mentions::{SymbolAliasSource, SymbolMention, SymbolMentionResolver};
use chrono::{DateTime, Utc};
use iced::widget::image::Handle as ImageHandle;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

const TELEGRAM_WEB_BASE: &str = "https://t.me/s/";
const TELEGRAM_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; Kerosene Telegram Feed; +https://github.com)";
const TELEGRAM_FEED_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const TELEGRAM_FEED_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const TELEGRAM_AVATAR_MAX_BODY_BYTES: usize = 512 * 1024;
pub(crate) const TELEGRAM_AVATAR_RETRY_BACKOFF_MS: u64 = 300_000;
pub(crate) const TELEGRAM_FEED_FETCH_LIMIT: usize = 10;
pub(crate) const TELEGRAM_FEED_RENDER_LIMIT: usize = 100;
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TelegramFeedPost {
    pub(crate) channel: String,
    pub(crate) message_id: u64,
    pub(crate) text: String,
    pub(crate) timestamp_ms: u64,
    pub(crate) fetched_at_ms: u64,
    pub(crate) request_started_ms: u64,
    pub(crate) request_duration_ms: u64,
    pub(crate) first_seen_ms: u64,
    pub(crate) url: String,
    pub(crate) ticker_mentions: Vec<TelegramTickerMention>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TelegramFastAuthOutcome {
    CodeSent,
    PasswordRequired { hint: Option<String> },
    SignedIn { display_name: String },
    SignedOut,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TelegramFastFeedEvent {
    Status {
        connected: bool,
        auth_required: bool,
        message: String,
    },
    Loaded(String, Box<Result<TelegramFeedPage, String>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TelegramFeedPrivateChannelConfig {
    pub peer_id: i64,
    pub title: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelegramPrivateChannelCandidate {
    pub(crate) peer_id: i64,
    pub(crate) title: String,
    pub(crate) avatar_handle: Option<ImageHandle>,
}

impl TelegramPrivateChannelCandidate {
    pub(crate) fn to_config(&self) -> TelegramFeedPrivateChannelConfig {
        TelegramFeedPrivateChannelConfig {
            peer_id: self.peer_id,
            title: self.title.clone(),
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
        self.request_duration_ms = request_duration_ms;
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TelegramFeedState {
    pub(crate) channels: Vec<String>,
    pub(crate) private_channels: Vec<TelegramFeedPrivateChannelConfig>,
    pub(crate) private_channel_candidates: Vec<TelegramPrivateChannelCandidate>,
    pub(crate) private_channel_candidates_loading: bool,
    pub(crate) private_channel_candidates_expanded: bool,
    pub(crate) notifications_enabled: bool,
    pub(crate) fast_mode_enabled: bool,
    pub(crate) fast_api_id: Option<i32>,
    pub(crate) fast_api_id_input: String,
    pub(crate) fast_api_hash_input: Zeroizing<String>,
    pub(crate) fast_phone_input: String,
    pub(crate) fast_code_input: Zeroizing<String>,
    pub(crate) fast_password_input: Zeroizing<String>,
    pub(crate) fast_auth_stage: TelegramFastAuthStage,
    pub(crate) fast_auth_in_flight: bool,
    pub(crate) fast_connected: bool,
    pub(crate) fast_status: Option<(String, bool)>,
    pub(crate) fast_password_hint: Option<String>,
    pub(crate) fast_reconnect_nonce: u64,
    pub(crate) fast_last_event_ms: Option<u64>,
    pub(crate) channels_expanded: bool,
    pub(crate) channel_input: String,
    pub(crate) channel_profiles: HashMap<String, TelegramChannelProfile>,
    pub(crate) posts: Vec<TelegramFeedPost>,
    ticker_mention_resolver: SymbolMentionResolver,
    seen_post_ids: HashMap<String, VecDeque<u64>>,
    pub(crate) loading_channels: Vec<String>,
    pub(crate) background_loading_channels: Vec<String>,
    pub(crate) next_avatar_request_id: u64,
    pub(crate) last_error: Option<String>,
    pub(crate) last_refresh_ms: Option<u64>,
}

impl TelegramFeedState {
    pub(crate) fn new(
        channels: &[String],
        private_channels: &[TelegramFeedPrivateChannelConfig],
        notifications_enabled: bool,
        fast_mode_enabled: bool,
        fast_api_id: Option<i32>,
    ) -> Self {
        Self {
            channels: normalized_channel_list(channels),
            private_channels: normalized_private_channel_list(private_channels),
            private_channel_candidates: Vec::new(),
            private_channel_candidates_loading: false,
            private_channel_candidates_expanded: false,
            notifications_enabled,
            fast_mode_enabled,
            fast_api_id,
            fast_api_id_input: fast_api_id
                .map(|api_id| api_id.to_string())
                .unwrap_or_default(),
            fast_api_hash_input: Zeroizing::new(String::new()),
            fast_phone_input: String::new(),
            fast_code_input: Zeroizing::new(String::new()),
            fast_password_input: Zeroizing::new(String::new()),
            fast_auth_stage: TelegramFastAuthStage::Idle,
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
            loading_channels: Vec::new(),
            background_loading_channels: Vec::new(),
            next_avatar_request_id: 0,
            last_error: None,
            last_refresh_ms: None,
        }
    }

    pub(crate) fn loading(&self) -> bool {
        !self.loading_channels.is_empty()
    }

    pub(crate) fn refreshing(&self) -> bool {
        self.loading()
            || !self.background_loading_channels.is_empty()
            || self.private_channel_candidates_loading
    }

    pub(crate) fn visible_posts(&self) -> Vec<TelegramFeedPost> {
        let mut posts = self.posts.clone();
        posts.sort_by(|left, right| {
            right
                .timestamp_ms
                .cmp(&left.timestamp_ms)
                .then_with(|| right.message_id.cmp(&left.message_id))
                .then_with(|| left.channel.cmp(&right.channel))
        });
        posts.truncate(TELEGRAM_FEED_RENDER_LIMIT);
        posts
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
}

pub(crate) fn default_telegram_feed_channels() -> Vec<String> {
    vec!["marketfeed".to_string()]
}

pub(crate) fn normalized_channel_list(channels: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for channel in channels {
        if let Ok(channel) = normalize_public_channel_input(channel)
            && !normalized.iter().any(|existing| existing == &channel)
        {
            normalized.push(channel);
        }
    }
    normalized
}

pub(crate) fn normalized_private_channel_list(
    channels: &[TelegramFeedPrivateChannelConfig],
) -> Vec<TelegramFeedPrivateChannelConfig> {
    let mut normalized = Vec::new();
    for channel in channels {
        if let Some(channel) = channel.normalized()
            && !normalized
                .iter()
                .any(|existing: &TelegramFeedPrivateChannelConfig| {
                    existing.peer_id == channel.peer_id
                })
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
    let request_started_ms = system_time_ms();
    let response = CLIENT
        .get(&url)
        .header(USER_AGENT, TELEGRAM_USER_AGENT)
        .timeout(TELEGRAM_FEED_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("@{channel} request failed: {e}"))?;
    let status = response.status();
    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("@{channel} response read failed: {e}"))?;
    let fetched_at_ms = system_time_ms();
    let request_duration_ms = fetched_at_ms.saturating_sub(request_started_ms);

    if body_bytes.len() > TELEGRAM_FEED_MAX_BODY_BYTES {
        return Err(format!(
            "@{channel} response was too large: {} bytes",
            body_bytes.len()
        ));
    }
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
    if response
        .content_length()
        .is_some_and(|len| len > TELEGRAM_AVATAR_MAX_BODY_BYTES as u64)
    {
        return Err(format!(
            "@{channel} avatar response was too large: more than {TELEGRAM_AVATAR_MAX_BODY_BYTES} bytes"
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let body = response
        .bytes()
        .await
        .map_err(|e| format!("@{channel} avatar response read failed: {e}"))?;
    if body.len() > TELEGRAM_AVATAR_MAX_BODY_BYTES {
        return Err(format!(
            "@{channel} avatar response was too large: {} bytes",
            body.len()
        ));
    }
    if !is_supported_raster_image(&body) {
        let content_type = content_type.unwrap_or_else(|| "unknown content type".to_string());
        return Err(format!(
            "@{channel} avatar response was not a supported image: {content_type}"
        ));
    }

    Ok(body.to_vec())
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
        .unwrap_or_else(|| channel_initials(&title, &channel));

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
        initials: channel_initials(&title, &channel),
        channel,
        title,
        avatar_url: None,
        avatar_handle: None,
        avatar_loading_url: None,
        avatar_request_id: 0,
        avatar_failed_at_ms: None,
    }
}

fn is_supported_raster_image(bytes: &[u8]) -> bool {
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
    let text = html_between(block, "tgme_widget_message_text js-message_text", "</div>")
        .map(|text_html| html_to_plain_text(&text_html))
        .unwrap_or_else(|| telegram_message_fallback_text(block));
    if text.trim().is_empty() {
        return None;
    }

    Some(TelegramFeedPost {
        channel: channel.to_string(),
        message_id,
        text,
        timestamp_ms,
        fetched_at_ms: 0,
        request_started_ms: 0,
        request_duration_ms: 0,
        first_seen_ms: 0,
        url: format!("https://t.me/{channel}/{message_id}"),
        ticker_mentions: Vec::new(),
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

fn channel_initials(title: &str, channel: &str) -> String {
    let mut initials = title
        .split_whitespace()
        .filter_map(|part| part.chars().find(|ch| ch.is_ascii_alphanumeric()))
        .take(2)
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>();
    if initials.is_empty() {
        initials = channel
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
    if first_seen_ms == 0 {
        return 0.0;
    }

    let age_ms = now_ms.saturating_sub(first_seen_ms);
    if age_ms >= TELEGRAM_NEW_MESSAGE_COOLDOWN_MS {
        0.0
    } else {
        1.0 - (age_ms as f32 / TELEGRAM_NEW_MESSAGE_COOLDOWN_MS as f32)
    }
}

pub(crate) fn telegram_arrival_latency_label(post: &TelegramFeedPost) -> Option<String> {
    if post.fetched_at_ms == 0 || post.first_seen_ms == 0 {
        return None;
    }

    Some(format!(
        "seen +{}",
        telegram_duration_label(post.fetched_at_ms.saturating_sub(post.timestamp_ms))
    ))
}

pub(crate) fn telegram_price_impact_pct(
    reference_price: Option<f64>,
    current_price: Option<f64>,
) -> Option<f64> {
    let reference = reference_price.filter(|price| price.is_finite() && *price > 0.0)?;
    let current = current_price.filter(|price| price.is_finite() && *price > 0.0)?;
    Some(((current / reference) - 1.0) * 100.0)
}

fn telegram_duration_label(duration_ms: u64) -> String {
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

fn system_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
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
    fn normalized_channel_list_dedupes_without_capping_channels() {
        let channels = (0..16)
            .map(|index| format!("channel_{index}"))
            .chain(std::iter::once("channel_1".to_string()))
            .collect::<Vec<_>>();

        let normalized = normalized_channel_list(&channels);

        assert_eq!(normalized.len(), 16);
        assert_eq!(normalized[0], "channel_0");
        assert_eq!(normalized[1], "channel_1");
        assert_eq!(normalized[15], "channel_15");
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

        assert_eq!(media_post.text, "[photo]");
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
            fetched_at_ms: 1_250,
            request_started_ms: 1_100,
            request_duration_ms: 150,
            first_seen_ms: 1_250,
            url: "https://t.me/marketfeed/1".to_string(),
            ticker_mentions: Vec::new(),
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
            fetched_at_ms: 9_000,
            request_started_ms: 8_850,
            request_duration_ms: 150,
            first_seen_ms: 0,
            url: "https://t.me/marketfeed/1".to_string(),
            ticker_mentions: Vec::new(),
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
}
