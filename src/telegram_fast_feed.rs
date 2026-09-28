mod auth;
mod media;
mod session;

pub(crate) use self::auth::{
    TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED, TELEGRAM_FAST_SESSION_CLEAR_FAILED,
    bundled_telegram_api_hash, bundled_telegram_api_id, clear_telegram_fast_pending_auth,
    clear_telegram_fast_pending_auth_except_request, clear_telegram_fast_pending_auth_for_request,
    request_telegram_fast_login_code, sign_out_telegram_fast, submit_telegram_fast_login_code,
    submit_telegram_fast_password,
};
#[cfg(test)]
pub(crate) use self::auth::{
    set_telegram_fast_pending_auth_placeholders_for_test,
    telegram_fast_pending_auth_request_ids_for_test, telegram_fast_pending_auth_test_lock,
};
pub(crate) use self::session::{clear_telegram_fast_session_files_at, telegram_fast_session_path};

use self::media::{
    download_private_channel_avatar_handle, fast_media_kind, spawn_fast_media_download,
};

use self::session::{
    open_telegram_session, prepare_session_path, tighten_session_permissions, with_telegram_client,
};

use crate::app_time::now_ms;
use crate::telegram_feed::{
    TELEGRAM_FAST_HEALTH_CHECK_INTERVAL_SECS, TELEGRAM_FEED_FETCH_LIMIT, TelegramChannelProfile,
    TelegramFastFeedEvent, TelegramFeedPage, TelegramFeedPost, TelegramFeedPostSource,
    TelegramFeedPrivateChannelConfig, TelegramPostMedia, TelegramPrivateChannelCandidate,
    normalize_private_channel_title, normalize_public_channel_input, normalize_telegram_plain_text,
    telegram_channel_profile_from_title, telegram_private_channel_peer_id_from_key,
};
use futures::{SinkExt as _, channel::mpsc};
use grammers_client::client::UpdatesConfiguration;
use grammers_client::peer::Peer;
use grammers_client::update::Update;
use grammers_client::{Client, SenderPool};
use grammers_session::types::{PeerId, PeerKind, PeerRef};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};

const TELEGRAM_FAST_UPDATE_QUEUE_LIMIT: usize = 2_000;
// Cap concurrent in-flight media downloads per session so a burst of media-heavy
// messages (or a slow backfill) cannot spawn an unbounded number of downloads.
const TELEGRAM_FAST_MEDIA_CONCURRENCY: usize = 4;
const TELEGRAM_FAST_RECONNECT_BACKFILL_LIMIT: usize = 100;
const TELEGRAM_FAST_RECONNECT_BASE_DELAY: Duration = Duration::from_secs(2);
const TELEGRAM_FAST_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);
const TELEGRAM_FAST_HEALTH_CHECK_INTERVAL: Duration =
    Duration::from_secs(TELEGRAM_FAST_HEALTH_CHECK_INTERVAL_SECS);
const TELEGRAM_PRIVATE_SCAN_TIMEOUT: Duration = Duration::from_secs(45);
const TELEGRAM_FAST_RESOLVE_RETRY_ATTEMPTS: usize = 2;
const TELEGRAM_FAST_RESOLVE_RETRY_DELAY: Duration = Duration::from_secs(10);
type ChannelIdMap = Arc<RwLock<HashMap<PeerId, FastChannelIdentity>>>;
type ChannelCursorMap = Arc<RwLock<HashMap<String, FastChannelCursor>>>;

/// Session-scoped shared handles threaded through channel resolution and
/// backfill, bundled to keep those functions' signatures small.
struct FastBackfillResources<'a> {
    channel_ids: &'a ChannelIdMap,
    channel_cursors: &'a ChannelCursorMap,
    media_semaphore: &'a Arc<Semaphore>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FastCursorGeneration {
    global: u64,
    channel: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FastChannelCursor {
    message_id: u64,
    generation: FastCursorGeneration,
}

#[derive(Debug, Default)]
struct FastCursorGenerations {
    global: u64,
    channels: HashMap<String, u64>,
}

// Cursors live for the whole process so that subscription restarts (reconnect
// nonce bumps, channel edits) keep gap-recovery backfill instead of falling
// back to the short initial-history fetch. Cleared on sign-out.
fn fast_channel_cursors() -> ChannelCursorMap {
    static CURSORS: OnceLock<ChannelCursorMap> = OnceLock::new();
    Arc::clone(CURSORS.get_or_init(|| Arc::new(RwLock::new(HashMap::new()))))
}

fn fast_cursor_generations() -> &'static Mutex<FastCursorGenerations> {
    static GENERATIONS: OnceLock<Mutex<FastCursorGenerations>> = OnceLock::new();
    GENERATIONS.get_or_init(|| Mutex::new(FastCursorGenerations::default()))
}

fn fast_cursor_generation(channel: &str) -> FastCursorGeneration {
    let Ok(generations) = fast_cursor_generations().lock() else {
        return FastCursorGeneration::default();
    };
    FastCursorGeneration {
        global: generations.global,
        channel: generations
            .channels
            .get(channel)
            .copied()
            .unwrap_or_default(),
    }
}

fn advance_fast_channel_cursor_generation(channel: &str) {
    let Ok(mut generations) = fast_cursor_generations().lock() else {
        return;
    };
    let entry = generations.channels.entry(channel.to_string()).or_default();
    *entry = entry.saturating_add(1);
}

fn advance_all_fast_cursor_generations() {
    let Ok(mut generations) = fast_cursor_generations().lock() else {
        return;
    };
    generations.global = generations.global.saturating_add(1);
    generations.channels.clear();
}

// Best-effort: a removed channel must not keep its cursor, or re-adding it
// would skip the initial history backfill. Contention is rare and losing the
// race only degrades to the old behavior.
pub(crate) fn clear_fast_channel_cursor(channel: &str) {
    advance_fast_channel_cursor_generation(channel);
    if let Ok(mut cursors) = fast_channel_cursors().try_write() {
        cursors.remove(channel);
    }
}

pub(crate) fn clear_all_fast_channel_cursors_best_effort() {
    advance_all_fast_cursor_generations();
    if let Ok(mut cursors) = fast_channel_cursors().try_write() {
        cursors.clear();
    }
}

#[cfg(test)]
pub(crate) async fn set_fast_channel_cursor_for_test(channel: &str, message_id: u64) {
    let generation = fast_cursor_generation(channel);
    fast_channel_cursors().write().await.insert(
        channel.to_string(),
        FastChannelCursor {
            message_id,
            generation,
        },
    );
}

#[cfg(test)]
pub(crate) async fn fast_channel_cursor_message_id_for_test(channel: &str) -> u64 {
    let generation = fast_cursor_generation(channel);
    channel_cursor_message_id(&fast_channel_cursors(), channel, generation).await
}

#[cfg(test)]
pub(crate) fn fast_channel_cursor_test_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

async fn clear_all_fast_channel_cursors() {
    advance_all_fast_cursor_generations();
    fast_channel_cursors().write().await.clear();
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl AbortOnDrop {
    fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self(handle)
    }

    fn abort(&self) {
        self.0.abort();
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct DropGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> DropGuard<F> {
    fn new(callback: F) -> Self {
        Self(Some(callback))
    }
}

impl<F: FnOnce()> Drop for DropGuard<F> {
    fn drop(&mut self) {
        if let Some(callback) = self.0.take() {
            callback();
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct TelegramFastFeedStreamParams {
    pub(crate) api_id: i32,
    pub(crate) channels: Vec<String>,
    pub(crate) private_channels: Vec<TelegramFeedPrivateChannelConfig>,
    pub(crate) reconnect_nonce: u64,
}

impl fmt::Debug for TelegramFastFeedStreamParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramFastFeedStreamParams")
            .field("api_id", &"<redacted>")
            .field("channels", &self.channels)
            .field(
                "private_channels",
                &RedactedPrivateChannelCount(self.private_channels.len()),
            )
            .field("reconnect_nonce", &self.reconnect_nonce)
            .finish()
    }
}

struct RedactedPrivateChannelCount(usize);

impl fmt::Debug for RedactedPrivateChannelCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} redacted>", self.0)
    }
}

#[derive(Clone, PartialEq, Eq)]
struct FastChannelIdentity {
    key: String,
    title: String,
    cursor_generation: FastCursorGeneration,
}

impl fmt::Debug for FastChannelIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private = telegram_private_channel_peer_id_from_key(&self.key).is_some();
        let key = if private {
            "<private>"
        } else {
            self.key.as_str()
        };
        let title = if private {
            "<redacted>"
        } else {
            self.title.as_str()
        };

        f.debug_struct("FastChannelIdentity")
            .field("key", &key)
            .field("title", &title)
            .field("cursor_generation", &self.cursor_generation)
            .finish()
    }
}

#[derive(Clone)]
struct FastChannelTarget {
    identity: FastChannelIdentity,
    profile: TelegramChannelProfile,
    peer_ref: PeerRef,
}

impl fmt::Debug for FastChannelTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FastChannelTarget")
            .field("identity", &self.identity)
            .field("profile", &self.profile)
            .field("peer_ref", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FastFeedSessionExit {
    Retry,
    Stop,
}

pub(crate) async fn list_telegram_private_channel_candidates(
    api_id: i32,
) -> Result<Vec<TelegramPrivateChannelCandidate>, String> {
    with_telegram_client(api_id, |client| async move {
        // The scan gates the feed's loading state; an unresponsive connection
        // must not leave it stuck forever.
        tokio::time::timeout(TELEGRAM_PRIVATE_SCAN_TIMEOUT, async move {
            if !client
                .is_authorized()
                .await
                .map_err(|_| "Telegram authorization check failed".to_string())?
            {
                return Err("Sign in to Telegram fast mode first".to_string());
            }

            let mut candidates = Vec::new();
            let mut dialogs = client.iter_dialogs().limit(500);
            while let Some(dialog) = dialogs
                .next()
                .await
                .map_err(|_| "Telegram channel list failed".to_string())?
            {
                let Peer::Channel(channel) = dialog.peer else {
                    continue;
                };
                if channel.username().is_some() {
                    continue;
                }
                let avatar_handle =
                    download_private_channel_avatar_handle(&client, Peer::Channel(channel.clone()))
                        .await;
                candidates.push(TelegramPrivateChannelCandidate {
                    peer_id: channel.id().bare_id(),
                    title: normalize_private_channel_title(channel.title(), channel.id().bare_id()),
                    avatar_handle,
                });
            }

            sort_and_dedup_private_channel_candidates(&mut candidates);
            Ok(candidates)
        })
        .await
        .unwrap_or_else(|_| Err("Telegram private channel scan timed out".to_string()))
    })
    .await
}

fn sort_and_dedup_private_channel_candidates(
    candidates: &mut Vec<TelegramPrivateChannelCandidate>,
) {
    candidates
        .sort_by_cached_key(|candidate| (candidate.title.to_ascii_lowercase(), candidate.peer_id));
    candidates.dedup_by_key(|candidate| candidate.peer_id);
}

pub(crate) fn telegram_fast_feed_stream(
    params: &TelegramFastFeedStreamParams,
) -> Pin<Box<dyn futures::Stream<Item = TelegramFastFeedEvent> + Send>> {
    let params = params.clone();
    Box::pin(iced::stream::channel(1000, async move |mut output| {
        let mut retry_delay = TELEGRAM_FAST_RECONNECT_BASE_DELAY;
        let channel_cursors = fast_channel_cursors();
        loop {
            let mut session_connected = false;
            match run_telegram_fast_feed_session(
                &params,
                Arc::clone(&channel_cursors),
                &mut session_connected,
                &mut output,
            )
            .await
            {
                FastFeedSessionExit::Retry => {
                    retry_delay = fast_retry_delay_after_session(retry_delay, session_connected);
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = next_fast_reconnect_delay(retry_delay);
                }
                FastFeedSessionExit::Stop => return,
            }
        }
    }))
}

async fn run_telegram_fast_feed_session(
    params: &TelegramFastFeedStreamParams,
    channel_cursors: ChannelCursorMap,
    session_connected: &mut bool,
    output: &mut mpsc::Sender<TelegramFastFeedEvent>,
) -> FastFeedSessionExit {
    let Some(session_path) = telegram_fast_session_path() else {
        let _ = send_status(
            output,
            false,
            true,
            "Could not resolve Kerosene config directory",
        )
        .await;
        return FastFeedSessionExit::Stop;
    };
    if let Err(err) = prepare_session_path(&session_path).await {
        let _ = send_status(output, false, true, &err).await;
        return FastFeedSessionExit::Stop;
    }

    let session = match open_telegram_session(&session_path).await {
        Ok(session) => Arc::new(session),
        Err(err) => {
            if !send_status(output, false, false, &err).await {
                return FastFeedSessionExit::Stop;
            }
            return FastFeedSessionExit::Retry;
        }
    };
    tighten_session_permissions(&session_path);

    let SenderPool {
        runner,
        updates,
        handle,
    } = SenderPool::new(session, params.api_id);
    let client = Client::new(handle.clone());
    let media_semaphore = Arc::new(Semaphore::new(TELEGRAM_FAST_MEDIA_CONCURRENCY));
    let _pool_task = AbortOnDrop::new(tokio::spawn(runner.run()));
    let handle_for_drop = handle.clone();
    let _handle_guard = DropGuard::new(move || {
        handle_for_drop.quit();
    });

    let authorized = match client.is_authorized().await {
        Ok(authorized) => authorized,
        Err(_) => {
            let _ = send_status(output, false, false, "Telegram authorization check failed").await;
            handle.quit();
            return FastFeedSessionExit::Retry;
        }
    };
    if !authorized {
        let _ = send_status(output, false, true, "Fast mode needs Telegram sign-in").await;
        handle.quit();
        return FastFeedSessionExit::Stop;
    }
    *session_connected = true;

    let mut updates = client
        .stream_updates(updates, live_first_updates_configuration())
        .await;
    let channels = normalized_channel_set(&params.channels);
    let private_channels = normalized_private_channel_map(&params.private_channels);
    let channel_cursor_generations =
        fast_cursor_generations_for_channels(&channels, &private_channels);
    let channel_ids = Arc::new(RwLock::new(HashMap::new()));
    let background_client = client.clone();
    let background_channels = channels.clone();
    let background_private_channels = private_channels.clone();
    let background_channel_cursor_generations = channel_cursor_generations.clone();
    let background_channel_ids = Arc::clone(&channel_ids);
    let background_channel_cursors = Arc::clone(&channel_cursors);
    let background_media_semaphore = Arc::clone(&media_semaphore);
    let mut background_output = output.clone();
    let background_task = AbortOnDrop::new(tokio::spawn(async move {
        let _ = send_status(
            &mut background_output,
            true,
            false,
            "Fast Telegram mode resolving channels",
        )
        .await;
        let backfill_resources = FastBackfillResources {
            channel_ids: &background_channel_ids,
            channel_cursors: &background_channel_cursors,
            media_semaphore: &background_media_semaphore,
        };
        let unresolved = resolve_and_backfill_fast_channels(
            &background_client,
            background_channels,
            background_private_channels,
            background_channel_cursor_generations,
            &backfill_resources,
            &mut background_output,
        )
        .await;
        warm_dialog_update_state(&background_client).await;
        let listening_status = if unresolved.is_empty() {
            "Fast Telegram mode listening".to_string()
        } else {
            format!(
                "Fast Telegram mode listening; could not resolve {}",
                unresolved.join(", ")
            )
        };
        let _ = send_status(&mut background_output, true, false, &listening_status).await;
    }));
    let health_client = client.clone();
    let health_handle = handle.clone();
    let mut health_output = output.clone();
    let health_task = AbortOnDrop::new(tokio::spawn(async move {
        loop {
            tokio::time::sleep(TELEGRAM_FAST_HEALTH_CHECK_INTERVAL).await;
            match health_client.is_authorized().await {
                Ok(true) => {
                    if !send_status(
                        &mut health_output,
                        true,
                        false,
                        "Fast Telegram mode listening",
                    )
                    .await
                    {
                        health_handle.quit();
                        return;
                    }
                }
                Ok(false) => {
                    let _ = send_status(
                        &mut health_output,
                        false,
                        true,
                        "Fast mode needs Telegram sign-in",
                    )
                    .await;
                    health_handle.quit();
                    return;
                }
                Err(_) => {
                    let _ = send_status(
                        &mut health_output,
                        false,
                        false,
                        "Telegram fast feed health check failed",
                    )
                    .await;
                    health_handle.quit();
                    return;
                }
            }
        }
    }));

    if !send_status(
        output,
        true,
        false,
        "Fast Telegram mode connected; preparing channel backfill",
    )
    .await
    {
        background_task.abort();
        health_task.abort();
        handle.quit();
        return FastFeedSessionExit::Stop;
    }

    loop {
        match updates.next().await {
            Ok(Update::NewMessage(message)) | Ok(Update::MessageEdited(message)) => {
                let Some(page) = fast_page_from_message(
                    &channels,
                    &private_channels,
                    &channel_cursor_generations,
                    &channel_ids,
                    &message,
                )
                .await
                else {
                    continue;
                };
                let channel = page.profile.channel.clone();
                let max_message_id = page
                    .posts
                    .iter()
                    .map(|post| post.message_id)
                    .max()
                    .unwrap_or_default();
                // Capture media that still needs its preview so it can be
                // downloaded off the hot update path and merged in afterward,
                // rather than stalling the live stream on a file fetch.
                let media_followup = page
                    .posts
                    .iter()
                    .find(|post| {
                        post.media
                            .as_ref()
                            .is_some_and(|media| media.handle.is_none())
                    })
                    .map(|post| (page.profile.clone(), post.clone()));
                if output
                    .send(TelegramFastFeedEvent::Loaded(
                        channel.clone(),
                        Box::new(Ok(page)),
                    ))
                    .await
                    .is_err()
                {
                    background_task.abort();
                    health_task.abort();
                    updates.sync_update_state().await;
                    handle.quit();
                    return FastFeedSessionExit::Stop;
                }
                let cursor_generation =
                    cursor_generation_for_channel(&channel_cursor_generations, &channel);
                record_channel_cursor(
                    &channel_cursors,
                    &channel,
                    max_message_id,
                    cursor_generation,
                )
                .await;
                if let Some((profile, post)) = media_followup {
                    // `message` is an update wrapper; clone the inner message it
                    // derefs to (the type the downloader expects).
                    spawn_fast_media_download(
                        &client,
                        &media_semaphore,
                        output,
                        channel.clone(),
                        profile,
                        post,
                        (*message).clone(),
                    );
                }
            }
            Ok(_) => {}
            Err(_) => {
                let _ = send_status(
                    output,
                    false,
                    false,
                    "Telegram fast feed disconnected; reconnecting",
                )
                .await;
                background_task.abort();
                health_task.abort();
                updates.sync_update_state().await;
                handle.quit();
                return FastFeedSessionExit::Retry;
            }
        }
    }
}

async fn send_status(
    output: &mut mpsc::Sender<TelegramFastFeedEvent>,
    connected: bool,
    auth_required: bool,
    message: &str,
) -> bool {
    output
        .send(status_event(connected, auth_required, message))
        .await
        .is_ok()
}

fn next_fast_reconnect_delay(current: Duration) -> Duration {
    current
        .saturating_mul(2)
        .min(TELEGRAM_FAST_RECONNECT_MAX_DELAY)
}

// A session that reached Telegram resets the backoff so reconnects after a
// long healthy run start from the base delay again.
fn fast_retry_delay_after_session(current: Duration, session_connected: bool) -> Duration {
    if session_connected {
        TELEGRAM_FAST_RECONNECT_BASE_DELAY
    } else {
        current
    }
}

fn live_first_updates_configuration() -> UpdatesConfiguration {
    UpdatesConfiguration {
        catch_up: false,
        update_queue_limit: Some(TELEGRAM_FAST_UPDATE_QUEUE_LIMIT),
    }
}

async fn warm_dialog_update_state(client: &Client) {
    let mut dialogs = client.iter_dialogs();
    let mut remaining = 250usize;
    while remaining > 0 {
        remaining -= 1;
        match dialogs.next().await {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
}

// Resolves channels and backfills each pass's targets immediately, retrying
// channels that failed to resolve (transient errors, FLOOD_WAIT) a bounded
// number of times. Returns display names of channels that never resolved.
async fn resolve_and_backfill_fast_channels(
    client: &Client,
    mut pending_channels: HashSet<String>,
    mut pending_private_channels: HashMap<i64, TelegramFeedPrivateChannelConfig>,
    channel_cursor_generations: HashMap<String, FastCursorGeneration>,
    resources: &FastBackfillResources<'_>,
    output: &mut mpsc::Sender<TelegramFastFeedEvent>,
) -> Vec<String> {
    let mut attempts = 0;
    loop {
        let targets = resolve_fast_channel_targets(
            client,
            &pending_channels,
            &pending_private_channels,
            &channel_cursor_generations,
            resources.channel_ids,
        )
        .await;
        let resolved: HashSet<String> = targets
            .iter()
            .map(|target| target.identity.key.clone())
            .collect();
        backfill_fast_channels(
            client,
            targets,
            Arc::clone(resources.channel_cursors),
            resources.media_semaphore,
            output,
        )
        .await;

        pending_channels.retain(|channel| !resolved.contains(channel));
        pending_private_channels.retain(|_, config| !resolved.contains(&config.key()));
        if pending_channels.is_empty() && pending_private_channels.is_empty() {
            return Vec::new();
        }
        if attempts >= TELEGRAM_FAST_RESOLVE_RETRY_ATTEMPTS {
            return pending_channels
                .iter()
                .map(|channel| format!("@{channel}"))
                .chain(
                    pending_private_channels
                        .values()
                        .map(|config| config.title.clone()),
                )
                .collect();
        }
        attempts += 1;
        tokio::time::sleep(TELEGRAM_FAST_RESOLVE_RETRY_DELAY).await;
    }
}

async fn resolve_fast_channel_targets(
    client: &Client,
    channels: &HashSet<String>,
    private_channels: &HashMap<i64, TelegramFeedPrivateChannelConfig>,
    channel_cursor_generations: &HashMap<String, FastCursorGeneration>,
    channel_ids: &ChannelIdMap,
) -> Vec<FastChannelTarget> {
    let mut targets = Vec::new();
    for channel in channels {
        if let Ok(Some(peer)) = client.resolve_username(channel).await {
            if !matches!(peer, Peer::Channel(_)) {
                continue;
            }
            let Some(peer_ref) = peer.to_ref().await else {
                continue;
            };
            let identity = FastChannelIdentity {
                key: channel.clone(),
                title: peer.name().unwrap_or(channel).to_string(),
                cursor_generation: cursor_generation_for_channel(
                    channel_cursor_generations,
                    channel,
                ),
            };
            channel_ids
                .write()
                .await
                .insert(peer.id(), identity.clone());
            targets.push(FastChannelTarget {
                profile: profile_from_identity(&identity, Some(&peer)),
                identity,
                peer_ref,
            });
        }
    }

    if private_channels.is_empty() {
        return targets;
    }

    let mut dialogs = client.iter_dialogs().limit(500);
    while let Ok(Some(dialog)) = dialogs.next().await {
        let Peer::Channel(channel) = dialog.peer else {
            continue;
        };
        let peer_id = channel.id().bare_id();
        let Some(config) = private_channels.get(&peer_id) else {
            continue;
        };
        let Some(peer_ref) = channel.to_ref().await else {
            continue;
        };
        let identity = FastChannelIdentity {
            key: config.key(),
            title: normalize_private_channel_title(channel.title(), peer_id),
            cursor_generation: cursor_generation_for_channel(
                channel_cursor_generations,
                &config.key(),
            ),
        };
        channel_ids
            .write()
            .await
            .insert(channel.id(), identity.clone());
        let mut profile = telegram_channel_profile_from_title(&identity.key, Some(&identity.title));
        profile.avatar_handle =
            download_private_channel_avatar_handle(client, Peer::Channel(channel.clone())).await;
        targets.push(FastChannelTarget {
            profile,
            identity,
            peer_ref,
        });
    }

    targets
}

async fn backfill_fast_channels(
    client: &Client,
    targets: Vec<FastChannelTarget>,
    channel_cursors: ChannelCursorMap,
    media_semaphore: &Arc<Semaphore>,
    output: &mut mpsc::Sender<TelegramFastFeedEvent>,
) {
    for target in targets {
        let mut posts = Vec::new();
        let mut media_jobs = Vec::new();
        let cursor = channel_cursor_message_id(
            &channel_cursors,
            &target.identity.key,
            target.identity.cursor_generation,
        )
        .await;
        // With a cursor, every backfilled message is newer than what was
        // already delivered, so it is live news arriving late, not history.
        let (limit, source) = if cursor == 0 {
            (
                TELEGRAM_FEED_FETCH_LIMIT,
                TelegramFeedPostSource::FastBackfill,
            )
        } else {
            (
                TELEGRAM_FAST_RECONNECT_BACKFILL_LIMIT,
                TelegramFeedPostSource::FastLive,
            )
        };
        let mut messages = client.iter_messages(target.peer_ref).limit(limit);
        loop {
            match messages.next().await {
                Ok(Some(message)) => {
                    let Some(post) = fast_post_from_message(&target.identity.key, &message, source)
                    else {
                        continue;
                    };
                    if cursor > 0 && post.message_id <= cursor {
                        break;
                    }
                    // Defer the preview download so a slow file never delays the
                    // rest of the backfilled page (text posts included).
                    if post.media.is_some() {
                        media_jobs.push((post.clone(), message));
                    }
                    posts.push(post);
                }
                Ok(None) => break,
                Err(_) => {
                    let _ = send_status(
                        output,
                        true,
                        false,
                        "Telegram backfill incomplete; continuing",
                    )
                    .await;
                    break;
                }
            }
        }
        posts.reverse();
        let max_message_id = posts
            .iter()
            .map(|post| post.message_id)
            .max()
            .unwrap_or_default();
        let profile = target.profile.clone();
        if !posts.is_empty()
            && output
                .send(TelegramFastFeedEvent::Loaded(
                    target.identity.key.clone(),
                    Box::new(Ok(TelegramFeedPage {
                        profile: target.profile,
                        posts,
                    })),
                ))
                .await
                .is_err()
        {
            return;
        }
        // The page (with placeholder media) is delivered; fill in each preview
        // off the critical path, merged back via follow-up events.
        for (post, message) in media_jobs {
            spawn_fast_media_download(
                client,
                media_semaphore,
                output,
                target.identity.key.clone(),
                profile.clone(),
                post,
                message,
            );
        }
        record_channel_cursor(
            &channel_cursors,
            &target.identity.key,
            max_message_id,
            target.identity.cursor_generation,
        )
        .await;
    }
}

async fn fast_page_from_message(
    channels: &HashSet<String>,
    private_channels: &HashMap<i64, TelegramFeedPrivateChannelConfig>,
    channel_cursor_generations: &HashMap<String, FastCursorGeneration>,
    channel_ids: &ChannelIdMap,
    message: &grammers_client::update::Message,
) -> Option<TelegramFeedPage> {
    let peer = message.peer();
    let public_channel = peer
        .and_then(|peer| match peer {
            Peer::Channel(_) => peer.username(),
            _ => None,
        })
        .and_then(|username| normalize_public_channel_input(username).ok())
        .filter(|channel| channels.contains(channel));
    let identity = if let Some(channel) = public_channel {
        FastChannelIdentity {
            title: channel.clone(),
            cursor_generation: cursor_generation_for_channel(channel_cursor_generations, &channel),
            key: channel,
        }
    } else if let Some(identity) = private_identity_for_peer_id(
        message.peer_id(),
        private_channels,
        channel_cursor_generations,
    ) {
        identity
    } else {
        channel_ids.read().await.get(&message.peer_id()).cloned()?
    };
    let profile = profile_from_identity(&identity, peer);
    let post = fast_post_from_message(&identity.key, message, TelegramFeedPostSource::FastLive)?;
    Some(TelegramFeedPage {
        profile,
        posts: vec![post],
    })
}

fn private_identity_for_peer_id(
    peer_id: PeerId,
    private_channels: &HashMap<i64, TelegramFeedPrivateChannelConfig>,
    channel_cursor_generations: &HashMap<String, FastCursorGeneration>,
) -> Option<FastChannelIdentity> {
    if peer_id.kind() != PeerKind::Channel {
        return None;
    }
    let config = private_channels.get(&peer_id.bare_id())?;
    let key = config.key();
    Some(FastChannelIdentity {
        cursor_generation: cursor_generation_for_channel(channel_cursor_generations, &key),
        key,
        title: config.title.clone(),
    })
}

fn fast_post_from_message(
    channel: &str,
    message: &grammers_client::message::Message,
    source: TelegramFeedPostSource,
) -> Option<TelegramFeedPost> {
    let message_id = u64::try_from(message.id()).ok()?;
    // The preview bytes are downloaded separately (inline for backfill, off the
    // hot path for live); this only records that a renderable preview exists.
    let media = message
        .media()
        .as_ref()
        .and_then(fast_media_kind)
        .map(TelegramPostMedia::placeholder);
    let mut text = normalize_telegram_plain_text(message.text());
    if text.trim().is_empty() {
        // A renderable preview stands in for the body; otherwise label the
        // media-only message so the card is not blank.
        text = if media.is_some() {
            String::new()
        } else {
            "[media]".to_string()
        };
    }
    let timestamp_ms = u64::try_from(message.date().timestamp_millis()).ok()?;
    let fetched_at_ms = now_ms();

    Some(TelegramFeedPost {
        channel: channel.to_string(),
        message_id,
        text,
        timestamp_ms,
        source,
        received_at_ms: fetched_at_ms,
        applied_at_ms: 0,
        fetched_at_ms,
        request_started_ms: fetched_at_ms,
        request_duration_ms: 0,
        first_seen_ms: if matches!(source, TelegramFeedPostSource::FastLive) {
            fetched_at_ms
        } else {
            0
        },
        url: telegram_post_url(channel, message_id),
        ticker_mentions: Vec::new(),
        media,
    })
}

fn profile_from_identity(
    identity: &FastChannelIdentity,
    peer: Option<&grammers_client::peer::Peer>,
) -> TelegramChannelProfile {
    telegram_channel_profile_from_title(
        &identity.key,
        peer.and_then(|peer| peer.name())
            .or(Some(identity.title.as_str())),
    )
}

fn normalized_channel_set(channels: &[String]) -> HashSet<String> {
    channels
        .iter()
        .filter_map(|channel| normalize_public_channel_input(channel).ok())
        .collect()
}

fn normalized_private_channel_map(
    channels: &[TelegramFeedPrivateChannelConfig],
) -> HashMap<i64, TelegramFeedPrivateChannelConfig> {
    channels
        .iter()
        .filter_map(TelegramFeedPrivateChannelConfig::normalized)
        .map(|channel| (channel.peer_id, channel))
        .collect()
}

fn fast_cursor_generations_for_channels(
    channels: &HashSet<String>,
    private_channels: &HashMap<i64, TelegramFeedPrivateChannelConfig>,
) -> HashMap<String, FastCursorGeneration> {
    channels
        .iter()
        .map(|channel| (channel.clone(), fast_cursor_generation(channel)))
        .chain(private_channels.values().map(|channel| {
            let key = channel.key();
            let generation = fast_cursor_generation(&key);
            (key, generation)
        }))
        .collect()
}

fn cursor_generation_for_channel(
    generations: &HashMap<String, FastCursorGeneration>,
    channel: &str,
) -> FastCursorGeneration {
    generations
        .get(channel)
        .copied()
        .unwrap_or_else(|| fast_cursor_generation(channel))
}

fn telegram_post_url(channel: &str, message_id: u64) -> String {
    telegram_private_channel_peer_id_from_key(channel)
        .map(|peer_id| format!("https://t.me/c/{peer_id}/{message_id}"))
        .unwrap_or_else(|| format!("https://t.me/{channel}/{message_id}"))
}

fn status_event(connected: bool, auth_required: bool, message: &str) -> TelegramFastFeedEvent {
    TelegramFastFeedEvent::Status {
        connected,
        auth_required,
        message: message.to_string(),
    }
}

async fn channel_cursor_message_id(
    channel_cursors: &ChannelCursorMap,
    channel: &str,
    generation: FastCursorGeneration,
) -> u64 {
    channel_cursors
        .read()
        .await
        .get(channel)
        .and_then(|cursor| (cursor.generation == generation).then_some(cursor.message_id))
        .unwrap_or_default()
}

async fn record_channel_cursor(
    channel_cursors: &ChannelCursorMap,
    channel: &str,
    message_id: u64,
    generation: FastCursorGeneration,
) {
    if message_id == 0 {
        return;
    }
    if fast_cursor_generation(channel) != generation {
        return;
    }
    let mut cursors = channel_cursors.write().await;
    if fast_cursor_generation(channel) != generation {
        cursors.remove(channel);
        return;
    }
    let entry = cursors
        .entry(channel.to_string())
        .or_insert(FastChannelCursor {
            message_id: 0,
            generation,
        });
    if entry.generation != generation {
        *entry = FastChannelCursor {
            message_id,
            generation,
        };
    } else {
        entry.message_id = entry.message_id.max(message_id);
    }
}

#[cfg(test)]
mod tests;
