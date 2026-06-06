use crate::app_time::now_ms;
use crate::telegram_feed::{
    TELEGRAM_FAST_HEALTH_CHECK_INTERVAL_SECS, TELEGRAM_FEED_FETCH_LIMIT, TelegramChannelProfile,
    TelegramFastAuthOutcome, TelegramFastFeedEvent, TelegramFeedPage, TelegramFeedPost,
    TelegramFeedPrivateChannelConfig, TelegramPrivateChannelCandidate,
    normalize_private_channel_title, normalize_public_channel_input, normalize_telegram_plain_text,
    telegram_channel_profile_from_title, telegram_private_channel_peer_id_from_key,
};
use futures::{SinkExt as _, channel::mpsc};
use grammers_client::client::{LoginToken, PasswordToken, UpdatesConfiguration};
use grammers_client::media::ChatPhoto;
use grammers_client::peer::Peer;
use grammers_client::session::storages::SqliteSession;
use grammers_client::update::Update;
use grammers_client::{Client, SenderPool, SignInError};
use grammers_session::types::{PeerId, PeerRef};
use iced::widget::image::Handle as ImageHandle;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;

const TELEGRAM_FAST_UPDATE_QUEUE_LIMIT: usize = 2_000;
const TELEGRAM_PRIVATE_CANDIDATE_AVATAR_MAX_BYTES: usize = 128 * 1024;
const TELEGRAM_FAST_RECONNECT_BASE_DELAY: Duration = Duration::from_secs(2);
const TELEGRAM_FAST_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);
const TELEGRAM_FAST_HEALTH_CHECK_INTERVAL: Duration =
    Duration::from_secs(TELEGRAM_FAST_HEALTH_CHECK_INTERVAL_SECS);
type ChannelIdMap = Arc<RwLock<HashMap<PeerId, FastChannelIdentity>>>;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TelegramFastFeedStreamParams {
    pub(crate) api_id: i32,
    pub(crate) channels: Vec<String>,
    pub(crate) private_channels: Vec<TelegramFeedPrivateChannelConfig>,
    pub(crate) reconnect_nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FastChannelIdentity {
    key: String,
    title: String,
}

#[derive(Debug, Clone)]
struct FastChannelTarget {
    identity: FastChannelIdentity,
    profile: TelegramChannelProfile,
    peer_ref: PeerRef,
}

enum PendingAuth {
    Login(LoginToken),
    Password(Box<PasswordToken>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FastFeedSessionExit {
    Retry,
    Stop,
}

fn pending_auths() -> &'static Mutex<HashMap<PathBuf, PendingAuth>> {
    static PENDING: OnceLock<Mutex<HashMap<PathBuf, PendingAuth>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn telegram_fast_session_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("kerosene").join("telegram_fast.session"))
}

pub(crate) fn bundled_telegram_api_id() -> Option<i32> {
    option_env!("KEROSENE_TELEGRAM_API_ID").and_then(|value| value.parse::<i32>().ok())
}

pub(crate) fn bundled_telegram_api_hash() -> Option<&'static str> {
    option_env!("KEROSENE_TELEGRAM_API_HASH").filter(|value| !value.trim().is_empty())
}

pub(crate) async fn request_telegram_fast_login_code(
    api_id: i32,
    api_hash: String,
    phone: String,
) -> Result<TelegramFastAuthOutcome, String> {
    let api_hash = api_hash.trim().to_string();
    let phone = phone.trim().to_string();
    if api_hash.is_empty() {
        return Err("Enter a Telegram API hash".to_string());
    }
    if phone.is_empty() {
        return Err("Enter a Telegram phone number".to_string());
    }

    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    with_telegram_client(api_id, |client| async move {
        if client
            .is_authorized()
            .await
            .map_err(|e| format!("Telegram authorization check failed: {e}"))?
        {
            return Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: "Telegram".to_string(),
            });
        }

        let token = client
            .request_login_code(&phone, &api_hash)
            .await
            .map_err(|e| format!("Telegram login code request failed: {e}"))?;
        if let Ok(mut pending) = pending_auths().lock() {
            pending.insert(session_path.clone(), PendingAuth::Login(token));
        }
        Ok(TelegramFastAuthOutcome::CodeSent)
    })
    .await
}

pub(crate) async fn submit_telegram_fast_login_code(
    api_id: i32,
    code: String,
) -> Result<TelegramFastAuthOutcome, String> {
    let code = code.trim().to_string();
    if code.is_empty() {
        return Err("Enter the Telegram login code".to_string());
    }

    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    let token = match pending_auths()
        .lock()
        .map_err(|_| "Telegram login state is unavailable".to_string())?
        .remove(&session_path)
    {
        Some(PendingAuth::Login(token)) => token,
        Some(PendingAuth::Password(password)) => {
            if let Ok(mut pending) = pending_auths().lock() {
                pending.insert(session_path, PendingAuth::Password(password));
            }
            return Err("Enter the Telegram 2FA password".to_string());
        }
        None => return Err("Request a Telegram login code first".to_string()),
    };

    with_telegram_client(api_id, |client| async move {
        match client.sign_in(&token, &code).await {
            Ok(user) => Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: user.first_name().unwrap_or("Telegram").to_string(),
            }),
            Err(SignInError::PasswordRequired(password)) => {
                let hint = password.hint().map(str::to_string);
                if let Some(session_path) = telegram_fast_session_path()
                    && let Ok(mut pending) = pending_auths().lock()
                {
                    pending.insert(session_path, PendingAuth::Password(Box::new(password)));
                }
                Ok(TelegramFastAuthOutcome::PasswordRequired { hint })
            }
            Err(err) => Err(format!("Telegram sign-in failed: {err}")),
        }
    })
    .await
}

pub(crate) async fn submit_telegram_fast_password(
    api_id: i32,
    password: String,
) -> Result<TelegramFastAuthOutcome, String> {
    if password.trim().is_empty() {
        return Err("Enter the Telegram 2FA password".to_string());
    }

    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    let token = match pending_auths()
        .lock()
        .map_err(|_| "Telegram login state is unavailable".to_string())?
        .remove(&session_path)
    {
        Some(PendingAuth::Password(token)) => *token,
        Some(PendingAuth::Login(login)) => {
            if let Ok(mut pending) = pending_auths().lock() {
                pending.insert(session_path, PendingAuth::Login(login));
            }
            return Err("Submit the Telegram login code first".to_string());
        }
        None => return Err("No Telegram 2FA challenge is pending".to_string()),
    };

    with_telegram_client(api_id, |client| async move {
        match client.check_password(token, password.as_bytes()).await {
            Ok(user) => Ok(TelegramFastAuthOutcome::SignedIn {
                display_name: user.first_name().unwrap_or("Telegram").to_string(),
            }),
            Err(SignInError::InvalidPassword(token)) => {
                let hint = token.hint().map(str::to_string);
                if let Some(session_path) = telegram_fast_session_path()
                    && let Ok(mut pending) = pending_auths().lock()
                {
                    pending.insert(session_path, PendingAuth::Password(Box::new(token)));
                }
                Err(format!(
                    "Telegram 2FA password was invalid{}",
                    hint.map(|hint| format!("; hint: {hint}"))
                        .unwrap_or_default()
                ))
            }
            Err(err) => Err(format!("Telegram 2FA sign-in failed: {err}")),
        }
    })
    .await
}

pub(crate) async fn sign_out_telegram_fast(api_id: i32) -> Result<TelegramFastAuthOutcome, String> {
    let result = with_telegram_client(api_id, |client| async move {
        if client
            .is_authorized()
            .await
            .map_err(|e| format!("Telegram authorization check failed: {e}"))?
        {
            let _ = client.sign_out().await;
        }
        Ok(TelegramFastAuthOutcome::SignedOut)
    })
    .await;
    clear_pending_auth();
    clear_telegram_fast_session_files();
    result
}

pub(crate) async fn list_telegram_private_channel_candidates(
    api_id: i32,
) -> Result<Vec<TelegramPrivateChannelCandidate>, String> {
    with_telegram_client(api_id, |client| async move {
        if !client
            .is_authorized()
            .await
            .map_err(|e| format!("Telegram authorization check failed: {e}"))?
        {
            return Err("Sign in to Telegram fast mode first".to_string());
        }

        let mut candidates = Vec::new();
        let mut dialogs = client.iter_dialogs().limit(500);
        while let Some(dialog) = dialogs
            .next()
            .await
            .map_err(|e| format!("Telegram channel list failed: {e}"))?
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

        candidates.sort_by(|left, right| {
            left.title
                .to_ascii_lowercase()
                .cmp(&right.title.to_ascii_lowercase())
                .then_with(|| left.peer_id.cmp(&right.peer_id))
        });
        candidates.dedup_by_key(|candidate| candidate.peer_id);
        Ok(candidates)
    })
    .await
}

async fn download_private_channel_avatar_handle(
    client: &Client,
    peer: Peer,
) -> Option<ImageHandle> {
    let photo = peer.photo(false).await?;
    let bytes = download_chat_photo_bytes(client, &photo).await?;
    Some(ImageHandle::from_bytes(bytes))
}

async fn download_chat_photo_bytes(client: &Client, photo: &ChatPhoto) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut download = client.iter_download(photo);
    while let Some(chunk) = download.next().await.ok()? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > TELEGRAM_PRIVATE_CANDIDATE_AVATAR_MAX_BYTES {
            return None;
        }
    }
    (!bytes.is_empty()).then_some(bytes)
}

pub(crate) fn telegram_fast_feed_stream(
    params: &TelegramFastFeedStreamParams,
) -> Pin<Box<dyn futures::Stream<Item = TelegramFastFeedEvent> + Send>> {
    let params = params.clone();
    Box::pin(iced::stream::channel(1000, async move |mut output| {
        let mut retry_delay = TELEGRAM_FAST_RECONNECT_BASE_DELAY;
        loop {
            match run_telegram_fast_feed_session(&params, &mut output).await {
                FastFeedSessionExit::Retry => {
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

    let session = match SqliteSession::open(&session_path).await {
        Ok(session) => Arc::new(session),
        Err(err) => {
            if !send_status(
                output,
                false,
                false,
                &format!("Telegram session open failed: {err}"),
            )
            .await
            {
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
    let _pool_task = AbortOnDrop::new(tokio::spawn(runner.run()));
    let handle_for_drop = handle.clone();
    let _handle_guard = DropGuard::new(move || {
        handle_for_drop.quit();
    });

    let authorized = match client.is_authorized().await {
        Ok(authorized) => authorized,
        Err(err) => {
            let _ = send_status(
                output,
                false,
                false,
                &format!("Telegram authorization check failed: {err}"),
            )
            .await;
            handle.quit();
            return FastFeedSessionExit::Retry;
        }
    };
    if !authorized {
        let _ = send_status(output, false, true, "Fast mode needs Telegram sign-in").await;
        handle.quit();
        return FastFeedSessionExit::Stop;
    }

    let mut updates = client
        .stream_updates(updates, live_first_updates_configuration())
        .await;
    let channels = normalized_channel_set(&params.channels);
    let private_channels = normalized_private_channel_map(&params.private_channels);
    let channel_ids = Arc::new(RwLock::new(HashMap::new()));
    let background_client = client.clone();
    let background_channels = channels.clone();
    let background_private_channels = private_channels.clone();
    let background_channel_ids = Arc::clone(&channel_ids);
    let mut background_output = output.clone();
    let background_task = AbortOnDrop::new(tokio::spawn(async move {
        let targets = resolve_fast_channel_targets(
            &background_client,
            &background_channels,
            &background_private_channels,
            &background_channel_ids,
        )
        .await;
        backfill_fast_channels(&background_client, targets, &mut background_output).await;
        warm_dialog_update_state(&background_client).await;
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
                Err(err) => {
                    let _ = send_status(
                        &mut health_output,
                        false,
                        false,
                        &format!("Telegram fast feed health check failed: {err}"),
                    )
                    .await;
                    health_handle.quit();
                    return;
                }
            }
        }
    }));

    if !send_status(output, true, false, "Fast Telegram mode listening").await {
        background_task.abort();
        health_task.abort();
        handle.quit();
        return FastFeedSessionExit::Stop;
    }

    loop {
        match updates.next().await {
            Ok(Update::NewMessage(message)) | Ok(Update::MessageEdited(message)) => {
                let Some(page) = fast_page_from_message(&channels, &channel_ids, &message).await
                else {
                    continue;
                };
                if output
                    .send(TelegramFastFeedEvent::Loaded(
                        page.profile.channel.clone(),
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
            }
            Ok(_) => {}
            Err(err) => {
                let _ = send_status(
                    output,
                    false,
                    false,
                    &format!("Telegram fast feed disconnected; reconnecting: {err}"),
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

fn clear_pending_auth() {
    if let Ok(mut pending) = pending_auths().lock() {
        pending.clear();
    }
}

fn live_first_updates_configuration() -> UpdatesConfiguration {
    UpdatesConfiguration {
        catch_up: false,
        update_queue_limit: Some(TELEGRAM_FAST_UPDATE_QUEUE_LIMIT),
    }
}

async fn with_telegram_client<T, F, Fut>(api_id: i32, f: F) -> Result<T, String>
where
    F: FnOnce(Client) -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let session_path = telegram_fast_session_path()
        .ok_or_else(|| "Could not resolve Kerosene config directory".to_string())?;
    prepare_session_path(&session_path).await?;
    let session = Arc::new(
        SqliteSession::open(&session_path)
            .await
            .map_err(|e| format!("Telegram session open failed: {e}"))?,
    );
    tighten_session_permissions(&session_path);

    let SenderPool {
        runner,
        updates: _,
        handle,
    } = SenderPool::new(session, api_id);
    let client = Client::new(handle.clone());
    let pool_task = tokio::spawn(runner.run());
    let result = f(client).await;
    handle.quit();
    let _ = pool_task.await;
    tighten_session_permissions(&session_path);
    result
}

async fn prepare_session_path(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create Telegram session directory: {e}"))?;
        tighten_directory_permissions(parent);
    }
    Ok(())
}

#[cfg(unix)]
fn tighten_directory_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn tighten_directory_permissions(_path: &Path) {}

#[cfg(unix)]
fn tighten_session_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    for candidate in session_file_family(path) {
        if candidate.exists() {
            let _ = std::fs::set_permissions(candidate, std::fs::Permissions::from_mode(0o600));
        }
    }
}

#[cfg(not(unix))]
fn tighten_session_permissions(_path: &Path) {}

fn clear_telegram_fast_session_files() {
    let Some(path) = telegram_fast_session_path() else {
        return;
    };
    for candidate in session_file_family(&path) {
        let _ = std::fs::remove_file(candidate);
    }
}

fn session_file_family(path: &Path) -> Vec<PathBuf> {
    vec![
        path.to_path_buf(),
        path.with_extension("session-shm"),
        path.with_extension("session-wal"),
        path.with_extension("session-journal"),
    ]
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

async fn resolve_fast_channel_targets(
    client: &Client,
    channels: &HashSet<String>,
    private_channels: &HashMap<i64, TelegramFeedPrivateChannelConfig>,
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
        };
        channel_ids
            .write()
            .await
            .insert(channel.id(), identity.clone());
        targets.push(FastChannelTarget {
            profile: telegram_channel_profile_from_title(&identity.key, Some(&identity.title)),
            identity,
            peer_ref,
        });
    }

    targets
}

async fn backfill_fast_channels(
    client: &Client,
    targets: Vec<FastChannelTarget>,
    output: &mut mpsc::Sender<TelegramFastFeedEvent>,
) {
    for target in targets {
        let mut posts = Vec::new();
        let mut messages = client
            .iter_messages(target.peer_ref)
            .limit(TELEGRAM_FEED_FETCH_LIMIT);
        while let Ok(Some(message)) = messages.next().await {
            if let Some(post) = fast_post_from_message(&target.identity.key, &message, false) {
                posts.push(post);
            }
        }
        posts.reverse();
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
    }
}

async fn fast_page_from_message(
    channels: &HashSet<String>,
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
            key: channel,
        }
    } else {
        channel_ids.read().await.get(&message.peer_id()).cloned()?
    };
    let profile = profile_from_identity(&identity, peer);
    let post = fast_post_from_message(&identity.key, message, true)?;
    Some(TelegramFeedPage {
        profile,
        posts: vec![post],
    })
}

fn fast_post_from_message(
    channel: &str,
    message: &grammers_client::message::Message,
    live_update: bool,
) -> Option<TelegramFeedPost> {
    let message_id = u64::try_from(message.id()).ok()?;
    let mut text = normalize_telegram_plain_text(message.text());
    if text.trim().is_empty() {
        text = "[media]".to_string();
    }
    let timestamp_ms = u64::try_from(message.date().timestamp_millis()).ok()?;
    let fetched_at_ms = now_ms();

    Some(TelegramFeedPost {
        channel: channel.to_string(),
        message_id,
        text,
        timestamp_ms,
        fetched_at_ms,
        request_started_ms: fetched_at_ms,
        request_duration_ms: 0,
        first_seen_ms: if live_update { fetched_at_ms } else { 0 },
        url: telegram_post_url(channel, message_id),
        ticker_mentions: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_updates_are_configured_live_first() {
        let config = live_first_updates_configuration();

        assert!(!config.catch_up);
        assert_eq!(
            config.update_queue_limit,
            Some(TELEGRAM_FAST_UPDATE_QUEUE_LIMIT)
        );
    }

    #[test]
    fn private_channel_configs_map_to_private_source_keys_and_links() {
        let channels = vec![TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Macro".to_string(),
        }];

        let mapped = normalized_private_channel_map(&channels);

        assert_eq!(
            mapped.get(&42).map(|channel| channel.key()).as_deref(),
            Some("private:42")
        );
        assert_eq!(telegram_post_url("private:42", 7), "https://t.me/c/42/7");
        assert_eq!(
            telegram_post_url("marketfeed", 7),
            "https://t.me/marketfeed/7"
        );
    }
}
