use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::config::KeroseneConfig;
use crate::message::TelegramFastAuthMessageResult;
use crate::telegram_fast_feed::{
    TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED, TELEGRAM_FAST_SESSION_CLEAR_FAILED,
    clear_telegram_fast_pending_auth, fast_channel_cursor_message_id_for_test,
    fast_channel_cursor_test_lock, set_fast_channel_cursor_for_test,
    set_telegram_fast_pending_auth_placeholders_for_test,
    telegram_fast_pending_auth_request_ids_for_test, telegram_fast_pending_auth_test_lock,
};
use crate::telegram_feed::{
    TelegramFastAuthOutcome, TelegramFastAuthStage, TelegramFastFeedEvent, TelegramMediaKind,
};

mod auth_requests;
mod ownership;

fn exchange_symbol(key: &str, ticker: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn sample_post(channel: &str, message_id: u64) -> TelegramFeedPost {
    TelegramFeedPost {
        channel: channel.to_string(),
        message_id,
        text: "sample".to_string(),
        timestamp_ms: 1_000,
        source: crate::telegram_feed::TelegramFeedPostSource::PublicPoll,
        received_at_ms: 1_100,
        applied_at_ms: 0,
        fetched_at_ms: 1_100,
        request_started_ms: 1_050,
        request_duration_ms: 50,
        first_seen_ms: 0,
        url: format!("https://t.me/{channel}/{message_id}"),
        ticker_mentions: Vec::new(),
        media: None,
    }
}

fn sample_post_with_media(
    channel: &str,
    message_id: u64,
    media: TelegramPostMedia,
) -> TelegramFeedPost {
    TelegramFeedPost {
        media: Some(media),
        ..sample_post(channel, message_id)
    }
}

fn find_post_media(
    terminal: &TradingTerminal,
    channel: &str,
    message_id: u64,
) -> TelegramPostMedia {
    terminal
        .telegram_feed
        .posts
        .iter()
        .find(|post| post.channel == channel && post.message_id == message_id)
        .and_then(|post| post.media.clone())
        .expect("post media")
}

#[test]
fn public_media_schedules_fetch_then_stores_handle() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

    let media =
        TelegramPostMedia::from_url(TelegramMediaKind::Photo, "https://cdn/p.jpg".to_string());
    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page(
            "marketfeed",
            vec![sample_post_with_media("marketfeed", 1, media)],
        )),
    );

    let pending = find_post_media(&terminal, "marketfeed", 1);
    assert_eq!(pending.loading_url.as_deref(), Some("https://cdn/p.jpg"));
    assert!(pending.handle.is_none());
    assert!(pending.request_id > 0);

    let _task = terminal.update_telegram_feed(Message::TelegramMediaLoaded(
        "marketfeed".to_string(),
        1,
        "https://cdn/p.jpg".to_string(),
        pending.request_id,
        Box::new(Ok(vec![0xFF, 0xD8, 0xFF, 0x00])),
    ));

    let loaded = find_post_media(&terminal, "marketfeed", 1);
    assert!(loaded.handle.is_some());
    assert!(loaded.loading_url.is_none());
    assert_eq!(loaded.request_id, 0);
    assert_eq!(loaded.failed_at_ms, None);
}

#[test]
fn stale_media_response_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

    let media =
        TelegramPostMedia::from_url(TelegramMediaKind::Photo, "https://cdn/p.jpg".to_string());
    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page(
            "marketfeed",
            vec![sample_post_with_media("marketfeed", 1, media)],
        )),
    );
    let request_id = find_post_media(&terminal, "marketfeed", 1).request_id;

    let _task = terminal.update_telegram_feed(Message::TelegramMediaLoaded(
        "marketfeed".to_string(),
        1,
        "https://cdn/p.jpg".to_string(),
        request_id + 999,
        Box::new(Ok(vec![0xFF, 0xD8, 0xFF, 0x00])),
    ));

    let media = find_post_media(&terminal, "marketfeed", 1);
    assert!(media.handle.is_none());
    assert_eq!(media.loading_url.as_deref(), Some("https://cdn/p.jpg"));
}

#[test]
fn refresh_keeps_loaded_media_and_refetches_changed_media() {
    // An unchanged reference keeps the already-downloaded preview.
    let mut existing = TelegramPostMedia::from_url(TelegramMediaKind::Photo, "same".to_string());
    existing.handle = Some(ImageHandle::from_bytes(vec![1, 2, 3]));
    let incoming = TelegramPostMedia::from_url(TelegramMediaKind::Photo, "same".to_string());
    let merged = merge_telegram_post_media(Some(existing), Some(incoming)).unwrap();
    assert!(merged.handle.is_some());

    // A freshly downloaded handle (fast follow-up) always wins over a placeholder.
    let placeholder = TelegramPostMedia::placeholder(TelegramMediaKind::Video);
    let mut downloaded = TelegramPostMedia::placeholder(TelegramMediaKind::Video);
    downloaded.handle = Some(ImageHandle::from_bytes(vec![9]));
    let merged = merge_telegram_post_media(Some(placeholder), Some(downloaded)).unwrap();
    assert!(merged.handle.is_some());

    // A changed public reference is replaced and must be re-fetched.
    let mut existing = TelegramPostMedia::from_url(TelegramMediaKind::Photo, "old".to_string());
    existing.handle = Some(ImageHandle::from_bytes(vec![1]));
    let incoming = TelegramPostMedia::from_url(TelegramMediaKind::Photo, "new".to_string());
    let merged = merge_telegram_post_media(Some(existing), Some(incoming)).unwrap();
    assert_eq!(merged.url.as_deref(), Some("new"));
    assert!(merged.handle.is_none());

    // A loaded fast-mode preview (no URL) survives a public refresh that
    // re-delivers the same message with a fetchable URL, so it is not dropped
    // and re-downloaded.
    let mut fast_loaded = TelegramPostMedia::placeholder(TelegramMediaKind::Photo);
    fast_loaded.handle = Some(ImageHandle::from_bytes(vec![7]));
    let public_incoming =
        TelegramPostMedia::from_url(TelegramMediaKind::Photo, "https://cdn/p.jpg".to_string());
    let merged = merge_telegram_post_media(Some(fast_loaded), Some(public_incoming)).unwrap();
    assert!(merged.handle.is_some());
    assert!(merged.url.is_none());

    // A failed download is surfaced onto a still-pending placeholder.
    let placeholder = TelegramPostMedia::placeholder(TelegramMediaKind::Video);
    let mut failed = TelegramPostMedia::placeholder(TelegramMediaKind::Video);
    failed.failed_at_ms = Some(42);
    let merged = merge_telegram_post_media(Some(placeholder), Some(failed)).unwrap();
    assert_eq!(merged.failed_at_ms, Some(42));
}

fn sample_profile(
    channel: &str,
    avatar_url: Option<&str>,
) -> crate::telegram_feed::TelegramChannelProfile {
    crate::telegram_feed::TelegramChannelProfile {
        channel: channel.to_string(),
        title: format!("@{channel}"),
        initials: channel.chars().take(2).collect(),
        avatar_url: avatar_url.map(str::to_string),
        avatar_handle: None,
        avatar_loading_url: None,
        avatar_request_id: 0,
        avatar_failed_at_ms: None,
    }
}

fn sample_page(channel: &str, posts: Vec<TelegramFeedPost>) -> TelegramFeedPage {
    TelegramFeedPage {
        profile: sample_profile(channel, None),
        posts,
    }
}

fn load_public_feed(
    terminal: &mut TradingTerminal,
    channel: &str,
    result: Result<TelegramFeedPage, String>,
) {
    let request_id = terminal.telegram_feed.begin_channel_refresh(channel);
    let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
        channel.to_string(),
        request_id,
        Box::new(result),
    ));
}

fn deliver_public_feed(
    terminal: &mut TradingTerminal,
    channel: &str,
    request_id: u64,
    result: Result<TelegramFeedPage, String>,
) {
    let _task = terminal.update_telegram_feed(Message::TelegramFeedLoaded(
        channel.to_string(),
        request_id,
        Box::new(result),
    ));
}

fn deliver_enabled_fast_feed_event(terminal: &mut TradingTerminal, event: TelegramFastFeedEvent) {
    terminal.telegram_feed.fast_mode_enabled = true;
    let reconnect_nonce = terminal.telegram_feed.fast_reconnect_nonce;
    let _task =
        terminal.update_telegram_feed(Message::TelegramFastFeedEvent(reconnect_nonce, event));
}

fn sample_page_with_avatar(
    channel: &str,
    avatar_url: &str,
    posts: Vec<TelegramFeedPost>,
) -> TelegramFeedPage {
    TelegramFeedPage {
        profile: sample_profile(channel, Some(avatar_url)),
        posts,
    }
}

#[test]
fn loaded_removed_channel_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.background_loading_channels = vec!["marketfeed".to_string()];
    let stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
    deliver_public_feed(
        &mut terminal,
        "marketfeed",
        stale_request_id,
        Ok(sample_page(
            "marketfeed",
            vec![sample_post("marketfeed", 1)],
        )),
    );

    assert!(terminal.telegram_feed.posts.is_empty());
    assert!(
        !terminal
            .telegram_feed
            .channel_profiles
            .contains_key("marketfeed")
    );
    assert_eq!(terminal.telegram_feed.last_error, None);
    assert!(terminal.telegram_feed.loading_channels.is_empty());
    assert!(
        terminal
            .telegram_feed
            .background_loading_channels
            .is_empty()
    );
}

#[test]
fn stale_public_channel_success_after_remove_and_readd_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    let stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    let _current_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

    deliver_public_feed(
        &mut terminal,
        "marketfeed",
        stale_request_id,
        Ok(sample_page(
            "marketfeed",
            vec![sample_post("marketfeed", 1)],
        )),
    );

    assert!(terminal.telegram_feed.posts.is_empty());
    assert!(
        !terminal
            .telegram_feed
            .channel_profiles
            .contains_key("marketfeed")
    );
    assert_eq!(
        terminal.telegram_feed.loading_channels,
        vec!["marketfeed".to_string()]
    );
}

#[test]
fn stale_public_channel_error_after_remove_and_readd_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    let stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    let _current_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");
    terminal.telegram_feed.last_error = Some("current status".to_string());

    deliver_public_feed(
        &mut terminal,
        "marketfeed",
        stale_request_id,
        Err("stale failure".to_string()),
    );

    assert_eq!(
        terminal.telegram_feed.last_error,
        Some("current status".to_string())
    );
    assert_eq!(
        terminal.telegram_feed.loading_channels,
        vec!["marketfeed".to_string()]
    );
}

#[test]
fn current_public_channel_error_redacts_last_error() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Err("telegram failed: api_hash=feed-secret".to_string()),
    );

    let error = terminal
        .telegram_feed
        .last_error
        .as_deref()
        .expect("telegram feed error");
    assert!(error.contains("api_hash=<redacted>"));
    assert!(!error.contains("feed-secret"));
}

#[test]
fn current_public_channel_success_after_remove_and_readd_is_applied() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    let _stale_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];
    let current_request_id = terminal.telegram_feed.begin_channel_refresh("marketfeed");

    deliver_public_feed(
        &mut terminal,
        "marketfeed",
        current_request_id,
        Ok(sample_page(
            "marketfeed",
            vec![sample_post("marketfeed", 1)],
        )),
    );

    assert_eq!(terminal.telegram_feed.posts.len(), 1);
    assert!(
        terminal
            .telegram_feed
            .channel_profiles
            .contains_key("marketfeed")
    );
    assert!(terminal.telegram_feed.loading_channels.is_empty());
}

#[test]
fn background_refresh_does_not_mark_visible_loading() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

    let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

    assert!(terminal.telegram_feed.loading_channels.is_empty());
    assert_eq!(
        terminal.telegram_feed.background_loading_channels,
        vec!["marketfeed".to_string()]
    );
}

#[test]
fn background_refresh_tick_skips_while_channel_fetch_in_flight() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.loading_channels = vec!["marketfeed".to_string()];

    let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

    assert!(
        terminal
            .telegram_feed
            .background_loading_channels
            .is_empty()
    );
}

#[test]
fn background_refresh_tick_runs_during_private_channel_scan() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.private_channel_candidates_loading = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

    assert_eq!(
        terminal.telegram_feed.background_loading_channels,
        vec!["marketfeed".to_string()]
    );
}

#[test]
fn fast_backfill_posts_do_not_alert_or_read_as_new() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.notifications_enabled = true;

    let mut live_post = sample_post("marketfeed", 11);
    live_post.source = crate::telegram_feed::TelegramFeedPostSource::FastLive;
    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Loaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![live_post]))),
        ),
    );
    assert!(terminal.toasts.is_empty());

    let mut backfill_post = sample_post("marketfeed", 10);
    backfill_post.source = crate::telegram_feed::TelegramFeedPostSource::FastBackfill;
    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Loaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![backfill_post]))),
        ),
    );

    let backfilled = terminal
        .telegram_feed
        .posts
        .iter()
        .find(|post| post.message_id == 10)
        .expect("backfilled post should be inserted");
    assert_eq!(backfilled.first_seen_ms, 0);
    assert!(terminal.toasts.is_empty());
}

#[test]
fn background_refresh_is_skipped_while_fast_feed_is_connected() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = true;
    terminal
        .telegram_feed
        .record_fast_connection_event(TradingTerminal::now_ms());

    let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

    assert!(terminal.telegram_feed.loading_channels.is_empty());
    assert!(
        terminal
            .telegram_feed
            .background_loading_channels
            .is_empty()
    );
}

#[test]
fn private_channel_scan_requires_signed_in_fast_session() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_api_id = Some(123);

    let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsRefresh);

    assert!(!terminal.telegram_feed.private_channel_candidates_loading);
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some(("Sign in to Telegram fast mode first".to_string(), true))
    );
}

#[test]
fn loaded_private_channel_candidates_open_and_can_collapse_selector() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    assert!(!terminal.telegram_feed.private_channel_candidates_expanded);
    let request_id = terminal
        .telegram_feed
        .next_private_channel_candidates_request_id();
    terminal.telegram_feed.private_channel_candidates_loading = true;

    let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(
        request_id,
        Box::new(Ok(vec![
            crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 42,
                title: "Private Macro".to_string(),
                avatar_handle: None,
            },
        ])),
    ));

    assert!(terminal.telegram_feed.private_channel_candidates_expanded);
    assert!(!terminal.telegram_feed.private_channel_candidates_loading);
    let _task =
        terminal.update_telegram_feed(Message::ToggleTelegramPrivateChannelCandidatesExpanded);
    assert!(!terminal.telegram_feed.private_channel_candidates_expanded);
}

#[test]
fn stale_private_channel_candidates_are_ignored_after_fast_mode_disable() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
    let stale_request_id = terminal
        .telegram_feed
        .next_private_channel_candidates_request_id();
    terminal.telegram_feed.private_channel_candidates_loading = true;

    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
    let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(
        stale_request_id,
        Box::new(Ok(vec![
            crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 42,
                title: "Private Macro".to_string(),
                avatar_handle: None,
            },
        ])),
    ));

    assert!(terminal.telegram_feed.private_channel_candidates.is_empty());
    assert!(!terminal.telegram_feed.private_channel_candidates_loading);
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some(("Fast mode disabled".to_string(), false))
    );
}

#[test]
fn stale_private_channel_candidates_are_ignored_after_newer_scan() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let stale_request_id = terminal
        .telegram_feed
        .next_private_channel_candidates_request_id();
    let _current_request_id = terminal
        .telegram_feed
        .next_private_channel_candidates_request_id();
    terminal.telegram_feed.private_channel_candidates =
        vec![crate::telegram_feed::TelegramPrivateChannelCandidate {
            peer_id: 7,
            title: "Current Private".to_string(),
            avatar_handle: None,
        }];
    terminal.telegram_feed.private_channel_candidates_loading = true;
    terminal.telegram_feed.fast_status = Some(("Scanning Telegram channels".to_string(), false));

    let _task = terminal.update_telegram_feed(Message::TelegramPrivateChannelsLoaded(
        stale_request_id,
        Box::new(Ok(vec![
            crate::telegram_feed::TelegramPrivateChannelCandidate {
                peer_id: 42,
                title: "Stale Private".to_string(),
                avatar_handle: None,
            },
        ])),
    ));

    assert_eq!(terminal.telegram_feed.private_channel_candidates.len(), 1);
    assert_eq!(
        terminal.telegram_feed.private_channel_candidates[0].title,
        "Current Private"
    );
    assert!(terminal.telegram_feed.private_channel_candidates_loading);
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some(("Scanning Telegram channels".to_string(), false))
    );
}

#[test]
fn stale_fast_feed_tick_reconnects_and_falls_back_to_public_refresh() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = true;
    terminal.telegram_feed.record_fast_connection_event(
        TradingTerminal::now_ms()
            .saturating_sub(crate::telegram_feed::TELEGRAM_FAST_STALE_AFTER_MS + 1),
    );
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;

    let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

    assert!(!terminal.telegram_feed.fast_connected);
    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        nonce.saturating_add(1)
    );
    assert_eq!(
        terminal.telegram_feed.background_loading_channels,
        vec!["marketfeed".to_string()]
    );
    assert!(
        terminal
            .telegram_feed
            .fast_status
            .as_ref()
            .is_some_and(|(status, is_error)| status.contains("stale") && *is_error)
    );
}

#[test]
fn fast_feed_toggle_is_persisted_without_disabling_public_channels() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];

    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);

    assert!(terminal.telegram_feed.fast_mode_enabled);
    assert_eq!(terminal.telegram_feed.channels, vec!["marketfeed"]);
}

#[test]
fn fast_feed_disable_abandons_pending_auth_challenge() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
    terminal.telegram_feed.fast_code_input = "12345".to_string().into();
    terminal.telegram_feed.fast_password_input = "password".to_string().into();
    terminal.telegram_feed.fast_password_hint = Some("hint".to_string());

    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);

    assert!(!terminal.telegram_feed.fast_mode_enabled);
    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::Idle
    );
    assert!(terminal.telegram_feed.fast_code_input.is_empty());
    assert!(terminal.telegram_feed.fast_password_input.is_empty());
    assert!(terminal.telegram_feed.fast_password_hint.is_none());
}

#[test]
fn stale_fast_auth_result_after_fast_mode_disable_is_ignored() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
    terminal.telegram_feed.fast_auth_in_flight = true;
    let stale_request_id = terminal.telegram_feed.next_fast_auth_request_id();

    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        stale_request_id,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
            display_name: "Alice".to_string(),
        })),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::Idle
    );
    assert!(!terminal.telegram_feed.fast_connected);
    assert!(!terminal.telegram_feed.fast_auth_in_flight);
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some(("Fast mode disabled".to_string(), false))
    );
}

#[test]
fn fast_feed_identity_edit_abandons_pending_auth_challenge() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
    terminal.telegram_feed.fast_auth_in_flight = true;
    terminal.telegram_feed.fast_code_input = "12345".to_string().into();
    terminal.telegram_feed.fast_password_hint = Some("hint".to_string());

    let _task =
        terminal.update_telegram_feed(Message::TelegramFastPhoneChanged("+15555550123".into()));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::Idle
    );
    assert!(terminal.telegram_feed.fast_code_input.is_empty());
    assert!(terminal.telegram_feed.fast_password_hint.is_none());
    assert!(!terminal.telegram_feed.fast_auth_in_flight);
}

#[test]
fn stale_fast_auth_result_after_identity_edit_is_ignored() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
    terminal.telegram_feed.fast_auth_in_flight = true;
    let stale_request_id = terminal.telegram_feed.next_fast_auth_request_id();

    let _task =
        terminal.update_telegram_feed(Message::TelegramFastPhoneChanged("+15555550123".into()));
    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        stale_request_id,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
            display_name: "Alice".to_string(),
        })),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::Idle
    );
    assert!(!terminal.telegram_feed.fast_connected);
    assert!(!terminal.telegram_feed.fast_auth_in_flight);
}

#[test]
fn stale_fast_code_sent_after_identity_edit_is_ignored() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
    terminal.telegram_feed.fast_auth_in_flight = true;
    let stale_request_id = terminal.telegram_feed.next_fast_auth_request_id();

    let _task =
        terminal.update_telegram_feed(Message::TelegramFastPhoneChanged("+15555550123".into()));

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        stale_request_id,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::CodeSent)),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::Idle
    );
    assert!(!terminal.telegram_feed.fast_auth_in_flight);
    assert!(!terminal.telegram_feed.fast_connected);
}

#[test]
fn fast_auth_error_status_is_sanitized() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let request_id = terminal.telegram_feed.next_fast_auth_request_id();
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        request_id,
        TelegramFastAuthMessageResult::new(Err(
            "telegram failed api_hash=hash-secret phone_code=code-secret".to_string(),
        )),
    ));

    let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
    assert_eq!(
        status,
        &("Telegram fast-mode request failed".to_string(), true)
    );
    assert!(!status.0.contains("hash-secret"));
    assert!(!status.0.contains("code-secret"));
}

#[test]
fn accepted_fast_code_sent_drops_abandoned_auth_challenges() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    clear_telegram_fast_pending_auth();
    set_telegram_fast_pending_auth_placeholders_for_test(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-a.session", 2),
        ("/tmp/kerosene-telegram-b.session", 3),
    ]);
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_auth_request_id = 2;
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        2,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::CodeSent)),
    ));

    assert_eq!(telegram_fast_pending_auth_request_ids_for_test(), vec![2]);
    clear_telegram_fast_pending_auth();
}

#[test]
fn fast_code_request_is_ignored_while_auth_request_is_in_flight() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_api_id = Some(12345);
    terminal.telegram_feed.fast_api_id_input = "12345".to_string();
    terminal.telegram_feed.fast_api_hash_input = "hash".to_string().into();
    terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
    let request_id = terminal.telegram_feed.next_fast_auth_request_id();
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastRequestCode);

    assert_eq!(terminal.telegram_feed.fast_auth_request_id, request_id);
    assert!(terminal.telegram_feed.fast_auth_in_flight);
    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::CodeRequested
    );
}

#[test]
fn fast_auth_local_session_clear_failure_does_not_mark_signed_out() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
    terminal.telegram_feed.fast_connected = true;
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;
    let request_id = terminal.telegram_feed.next_fast_auth_request_id();
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        request_id,
        TelegramFastAuthMessageResult::new(Err(format!(
            "{TELEGRAM_FAST_SESSION_CLEAR_FAILED}: remove <config-dir>/telegram_fast.session failed"
        ))),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::SignedIn
    );
    assert!(terminal.telegram_feed.fast_connected);
    assert_eq!(terminal.telegram_feed.fast_reconnect_nonce, nonce);
    let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
    assert_eq!(
        status,
        &(TELEGRAM_FAST_SESSION_CLEAR_FAILED.to_string(), true)
    );
    assert!(!status.0.contains("/tmp/kerosene"));
    assert!(!terminal.telegram_feed.fast_auth_in_flight);
}

#[test]
fn fast_auth_signed_out_warning_clears_local_runtime_state() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
    terminal.telegram_feed.fast_connected = true;
    terminal
        .telegram_feed
        .record_fast_connection_event(TradingTerminal::now_ms());
    terminal.telegram_feed.fast_code_input = "12345".to_string().into();
    terminal.telegram_feed.fast_password_input = "password".to_string().into();
    terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;
    let request_id = terminal.telegram_feed.next_fast_auth_request_id();
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        request_id,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedOut {
            warning: Some(TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string()),
        })),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::Idle
    );
    assert!(!terminal.telegram_feed.fast_connected);
    assert!(terminal.telegram_feed.fast_last_event_ms.is_none());
    assert!(terminal.telegram_feed.fast_code_input.is_empty());
    assert!(terminal.telegram_feed.fast_password_input.is_empty());
    assert!(terminal.telegram_feed.fast_phone_input.is_empty());
    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        nonce.saturating_add(1)
    );
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some((TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string(), true))
    );
}

#[test]
fn fast_password_required_status_does_not_include_hint() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let request_id = terminal.telegram_feed.next_fast_auth_request_id();
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        request_id,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::PasswordRequired {
            hint: Some("hint-secret".to_string()),
        })),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_password_hint.as_deref(),
        Some("hint-secret")
    );
    let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
    assert_eq!(
        status,
        &("Telegram 2FA password required".to_string(), false)
    );
    assert!(!status.0.contains("hint-secret"));
}

#[test]
fn fast_stream_status_message_is_sanitized() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Status {
            connected: true,
            auth_required: false,
            message: "connected api_hash=hash-secret phone_code=code-secret".to_string(),
        },
    );

    let status = terminal.telegram_feed.fast_status.as_ref().expect("status");
    assert_eq!(status, &("Fast Telegram mode listening".to_string(), false));
    assert!(!status.0.contains("hash-secret"));
    assert!(!status.0.contains("code-secret"));
}

#[test]
fn fast_feed_signed_in_result_clears_login_inputs_and_reconnects() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_api_hash_input = "hash".to_string().into();
    terminal.telegram_feed.fast_phone_input = "+15555550123".to_string();
    terminal.telegram_feed.fast_code_input = "12345".to_string().into();
    terminal.telegram_feed.fast_password_input = "password".to_string().into();
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;
    let request_id = terminal.telegram_feed.next_fast_auth_request_id();
    terminal.telegram_feed.fast_auth_in_flight = true;

    let _task = terminal.update_telegram_feed(Message::TelegramFastAuthResult(
        request_id,
        TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
            display_name: "Alice".to_string(),
        })),
    ));

    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::SignedIn
    );
    assert!(terminal.telegram_feed.fast_connected);
    assert!(terminal.telegram_feed.fast_last_event_ms.is_some());
    assert!(terminal.telegram_feed.fast_api_hash_input.is_empty());
    assert!(terminal.telegram_feed.fast_phone_input.is_empty());
    assert!(terminal.telegram_feed.fast_code_input.is_empty());
    assert!(terminal.telegram_feed.fast_password_input.is_empty());
    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        nonce.saturating_add(1)
    );
}

#[test]
fn fast_feed_status_heartbeat_keeps_connection_fresh() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = true;
    terminal.telegram_feed.record_fast_connection_event(
        TradingTerminal::now_ms()
            .saturating_sub(crate::telegram_feed::TELEGRAM_FAST_STALE_AFTER_MS + 1),
    );

    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Status {
            connected: true,
            auth_required: false,
            message: "Fast Telegram mode listening".to_string(),
        },
    );

    assert!(terminal.telegram_feed.fast_connected);
    assert!(
        !terminal
            .telegram_feed
            .fast_connection_stale(TradingTerminal::now_ms())
    );
}

#[test]
fn stale_fast_feed_status_after_reconnect_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = false;
    let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;
    terminal.telegram_feed.fast_reconnect_nonce = stale_nonce.saturating_add(1);

    let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
        stale_nonce,
        TelegramFastFeedEvent::Status {
            connected: true,
            auth_required: false,
            message: "Fast Telegram mode listening".to_string(),
        },
    ));

    assert!(!terminal.telegram_feed.fast_connected);
    assert_eq!(terminal.telegram_feed.fast_status, None);
}

#[test]
fn adding_public_channel_invalidates_stale_fast_status() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = true;
    terminal
        .telegram_feed
        .record_fast_connection_event(TradingTerminal::now_ms());
    terminal.telegram_feed.fast_status = Some(("Fast Telegram mode listening".to_string(), false));
    terminal.telegram_feed.channel_input = "freshfeed".to_string();
    let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;

    let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        stale_nonce.saturating_add(1)
    );
    assert!(!terminal.telegram_feed.fast_connected);
    assert!(terminal.telegram_feed.fast_last_event_ms.is_none());

    let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
        stale_nonce,
        TelegramFastFeedEvent::Status {
            connected: true,
            auth_required: false,
            message: "Fast Telegram mode listening".to_string(),
        },
    ));

    assert!(!terminal.telegram_feed.fast_connected);
    assert!(
        terminal
            .telegram_feed
            .fast_status
            .as_ref()
            .is_some_and(|(status, is_error)| {
                status.contains("channel list changed") && !*is_error
            })
    );
}

#[test]
fn removing_public_channel_allows_background_refresh_after_fast_status_invalidated() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string(), "otherfeed".to_string()];
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = true;
    terminal
        .telegram_feed
        .record_fast_connection_event(TradingTerminal::now_ms());
    let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));

    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        stale_nonce.saturating_add(1)
    );
    assert!(!terminal.telegram_feed.fast_connected);
    assert!(terminal.telegram_feed.fast_last_event_ms.is_none());

    let _task = terminal.update_telegram_feed(Message::TelegramFeedRefreshTick);

    assert_eq!(
        terminal.telegram_feed.background_loading_channels,
        vec!["otherfeed".to_string()]
    );

    let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
        stale_nonce,
        TelegramFastFeedEvent::Status {
            connected: true,
            auth_required: false,
            message: "Fast Telegram mode listening".to_string(),
        },
    ));

    assert!(!terminal.telegram_feed.fast_connected);
}

#[test]
fn fast_feed_loaded_event_after_disable_is_ignored() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.fast_mode_enabled = true;
    let stale_nonce = terminal.telegram_feed.fast_reconnect_nonce;
    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
    let mut post = sample_post("marketfeed", 22);
    post.source = crate::telegram_feed::TelegramFeedPostSource::FastLive;

    let _task = terminal.update_telegram_feed(Message::TelegramFastFeedEvent(
        stale_nonce,
        TelegramFastFeedEvent::Loaded(
            "marketfeed".to_string(),
            Box::new(Ok(sample_page("marketfeed", vec![post]))),
        ),
    ));

    assert!(!terminal.telegram_feed.fast_mode_enabled);
    assert!(terminal.telegram_feed.posts.is_empty());
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some(("Fast mode disabled".to_string(), false))
    );
}

#[test]
fn fast_feed_disconnect_status_marks_fast_feed_disconnected() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_mode_enabled = true;
    terminal.telegram_feed.fast_connected = true;
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;

    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Status {
            connected: false,
            auth_required: false,
            message: "Telegram fast feed disconnected; reconnecting".to_string(),
        },
    );

    assert!(!terminal.telegram_feed.fast_connected);
    assert_eq!(terminal.telegram_feed.fast_reconnect_nonce, nonce);
    assert_eq!(
        terminal.telegram_feed.fast_auth_stage,
        TelegramFastAuthStage::SignedIn
    );
}

#[test]
fn fast_feed_request_code_is_ignored_when_signed_in() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.fast_connected = true;
    terminal.telegram_feed.fast_auth_stage = TelegramFastAuthStage::SignedIn;

    let _task = terminal.update_telegram_feed(Message::TelegramFastRequestCode);

    assert!(!terminal.telegram_feed.fast_auth_in_flight);
    assert_eq!(
        terminal.telegram_feed.fast_status,
        Some(("Fast mode is already signed in".to_string(), false))
    );
}

#[test]
fn refreshing_existing_post_preserves_row_timing() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    let mut initial_post = sample_post("marketfeed", 1);
    initial_post.fetched_at_ms = 1_100;

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![initial_post])),
    );

    let mut refreshed_post = sample_post("marketfeed", 1);
    refreshed_post.text = "edited".to_string();
    refreshed_post.fetched_at_ms = 9_999;
    refreshed_post.request_duration_ms = 999;
    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![refreshed_post])),
    );

    assert_eq!(terminal.telegram_feed.posts.len(), 1);
    let post = &terminal.telegram_feed.posts[0];
    assert_eq!(post.text, "edited");
    assert_eq!(post.fetched_at_ms, 1_100);
    assert_eq!(post.request_duration_ms, 50);
}

#[test]
fn loaded_posts_capture_ticker_mentions_with_reference_price() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
    terminal
        .telegram_feed
        .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    // The baseline is anchored to a mid recorded at/before the message time
    // (timestamp_ms 1_000), not to whenever the app noticed the post.
    terminal.record_screener_mid_samples(
        &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
        500,
    );
    let mut post = sample_post("marketfeed", 1);
    post.text = "BTC is moving".to_string();

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![post])),
    );

    let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].symbol, "BTC");
    assert_eq!(mentions[0].ticker, "BTC");
    assert_eq!(mentions[0].matched_text, "BTC");
    assert_eq!(
        mentions[0].source,
        crate::symbol_mentions::SymbolAliasSource::Ticker
    );
    assert_eq!(mentions[0].reference_price, Some(100.0));
}

#[test]
fn refreshing_existing_post_preserves_ticker_reference_price() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
    terminal
        .telegram_feed
        .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    terminal.record_screener_mid_samples(
        &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
        500,
    );
    let mut initial = sample_post("marketfeed", 1);
    initial.text = "BTC is moving".to_string();

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![initial])),
    );
    terminal.all_mids.insert("BTC".to_string(), 105.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    let mut refreshed = sample_post("marketfeed", 1);
    refreshed.text = "edited BTC".to_string();

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![refreshed])),
    );

    let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].reference_price, Some(100.0));
}

#[test]
fn reference_price_anchors_to_message_time_not_late_mid() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
    terminal
        .telegram_feed
        .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
    // Pre-news price recorded at/just before the message timestamp (1_000).
    terminal.record_screener_mid_samples(
        &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
        500,
    );
    // The market has already moved by the time we see the post; the current
    // (late) mid must NOT become the baseline.
    terminal.all_mids.insert("BTC".to_string(), 200.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    let mut post = sample_post("marketfeed", 1);
    post.text = "BTC is moving".to_string();

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![post])),
    );

    let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
    assert_eq!(mentions[0].reference_price, Some(100.0));
}

#[test]
fn reference_price_suppressed_for_old_post_without_message_time_sample() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
    terminal
        .telegram_feed
        .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
    // A fresh mid exists, but the post is ancient (timestamp 1_000) and there
    // is no recorded mid at message time, so anchoring to it would mislead.
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    let mut post = sample_post("marketfeed", 1);
    post.text = "BTC is moving".to_string();

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", vec![post])),
    );

    let mentions = &terminal.telegram_feed.posts[0].ticker_mentions;
    assert_eq!(mentions.len(), 1);
    assert_eq!(mentions[0].reference_price, None);
    // No price captured => no baseline timestamp stamped.
    assert_eq!(mentions[0].reference_seen_ms, 0);
}

#[test]
fn fill_missing_reference_uses_recent_fallback_and_is_immutable() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
    let mut post = sample_post("marketfeed", 1); // timestamp_ms = 1_000
    post.ticker_mentions = vec![TelegramTickerMention {
        symbol: "BTC".to_string(),
        ticker: "BTC".to_string(),
        matched_text: "BTC".to_string(),
        source: crate::symbol_mentions::SymbolAliasSource::Ticker,
        confidence: 100,
        reference_price: None,
        reference_seen_ms: 0,
    }];
    terminal.telegram_feed.posts = vec![post];

    // No message-time sample, but the post is recent relative to `now`, so the
    // current fresh mid is an acceptable baseline.
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), 50_000);
    terminal.fill_missing_telegram_ticker_reference_prices(60_000);
    let mention = &terminal.telegram_feed.posts[0].ticker_mentions[0];
    assert_eq!(mention.reference_price, Some(100.0));
    assert_eq!(mention.reference_seen_ms, 60_000);

    // A later mids tick must NOT rebase an already-captured reference.
    terminal.all_mids.insert("BTC".to_string(), 200.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), 65_000);
    terminal.fill_missing_telegram_ticker_reference_prices(70_000);
    let mention = &terminal.telegram_feed.posts[0].ticker_mentions[0];
    assert_eq!(mention.reference_price, Some(100.0));
    assert_eq!(mention.reference_seen_ms, 60_000);
}

#[test]
fn fast_profile_update_without_avatar_keeps_cached_avatar() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channel_profiles.insert(
        "marketfeed".to_string(),
        crate::telegram_feed::TelegramChannelProfile {
            channel: "marketfeed".to_string(),
            title: "@marketfeed".to_string(),
            initials: "MA".to_string(),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            avatar_handle: Some(ImageHandle::from_bytes(vec![0x89, b'P', b'N', b'G'])),
            avatar_loading_url: None,
            avatar_request_id: 42,
            avatar_failed_at_ms: None,
        },
    );

    let _task =
        terminal.store_telegram_channel_profile(crate::telegram_feed::TelegramChannelProfile {
            channel: "marketfeed".to_string(),
            title: "Market Feed".to_string(),
            initials: "MF".to_string(),
            avatar_url: None,
            avatar_handle: None,
            avatar_loading_url: None,
            avatar_request_id: 0,
            avatar_failed_at_ms: None,
        });

    let profile = terminal
        .telegram_feed
        .channel_profiles
        .get("marketfeed")
        .expect("profile should remain cached");
    assert_eq!(
        profile.avatar_url.as_deref(),
        Some("https://example.com/avatar.jpg")
    );
    assert!(profile.avatar_handle.is_some());
    assert_eq!(profile.title, "Market Feed");
}

#[tokio::test]
async fn removing_channel_clears_cached_profile_and_fast_cursor() {
    let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    set_fast_channel_cursor_for_test("marketfeed", 99).await;
    assert_eq!(
        fast_channel_cursor_message_id_for_test("marketfeed").await,
        99
    );
    terminal.telegram_feed.channel_profiles.insert(
        "marketfeed".to_string(),
        sample_profile("marketfeed", Some("https://example.com/avatar.jpg")),
    );

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel("marketfeed".into()));

    assert!(
        !terminal
            .telegram_feed
            .channels
            .contains(&"marketfeed".to_string())
    );
    assert!(
        !terminal
            .telegram_feed
            .channel_profiles
            .contains_key("marketfeed")
    );
    assert_eq!(
        fast_channel_cursor_message_id_for_test("marketfeed").await,
        0
    );
}

#[test]
fn adding_private_channel_uses_scanned_candidate_and_reconnects_fast_feed() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.private_channel_candidates.push(
        crate::telegram_feed::TelegramPrivateChannelCandidate {
            peer_id: 42,
            title: "Private Macro".to_string(),
            avatar_handle: Some(ImageHandle::from_bytes(vec![0x89, b'P', b'N', b'G'])),
        },
    );
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;

    let _task = terminal.update_telegram_feed(Message::TelegramFeedAddPrivateChannel(42));

    assert_eq!(terminal.telegram_feed.private_channels.len(), 1);
    assert_eq!(terminal.telegram_feed.private_channels[0].peer_id, 42);
    assert_eq!(
        terminal.telegram_feed.private_channels[0].title,
        "Private Macro"
    );
    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        nonce.saturating_add(1)
    );
    let profile = terminal
        .telegram_feed
        .channel_profiles
        .get("private:42")
        .expect("selected private channel should cache scanned profile");
    assert_eq!(profile.title, "Private Macro");
    assert!(profile.avatar_handle.is_some());
}

#[tokio::test]
async fn adding_private_channel_clears_existing_fast_cursor() {
    let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let key = crate::telegram_feed::telegram_private_channel_key(42);
    set_fast_channel_cursor_for_test(&key, 99).await;
    assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 99);
    terminal.telegram_feed.private_channel_candidates.push(
        crate::telegram_feed::TelegramPrivateChannelCandidate {
            peer_id: 42,
            title: "Private Macro".to_string(),
            avatar_handle: None,
        },
    );

    let _task = terminal.update_telegram_feed(Message::TelegramFeedAddPrivateChannel(42));

    assert_eq!(terminal.telegram_feed.private_channels.len(), 1);
    assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 0);
}

#[test]
fn selected_private_channel_fast_event_is_inserted() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let key = crate::telegram_feed::telegram_private_channel_key(42);
    terminal.telegram_feed.private_channels =
        vec![crate::telegram_feed::TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Macro".to_string(),
        }];
    let mut post = sample_post(&key, 7);
    post.source = crate::telegram_feed::TelegramFeedPostSource::FastLive;
    post.received_at_ms = 1_100;
    post.first_seen_ms = 1_100;

    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Loaded(key.clone(), Box::new(Ok(sample_page(&key, vec![post])))),
    );

    assert_eq!(terminal.telegram_feed.posts.len(), 1);
    assert_eq!(terminal.telegram_feed.posts[0].channel, key);
    assert_eq!(terminal.telegram_feed.posts[0].message_id, 7);
    assert_eq!(
        terminal.telegram_feed.posts[0].source,
        crate::telegram_feed::TelegramFeedPostSource::FastLive
    );
    assert_eq!(terminal.telegram_feed.posts[0].received_at_ms, 1_100);
    assert!(terminal.telegram_feed.posts[0].applied_at_ms > 0);
}

#[test]
fn fast_profile_update_with_private_avatar_keeps_incoming_handle() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let key = crate::telegram_feed::telegram_private_channel_key(42);
    terminal.telegram_feed.private_channels =
        vec![crate::telegram_feed::TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Macro".to_string(),
        }];
    terminal
        .telegram_feed
        .channel_profiles
        .insert(key.clone(), sample_profile(&key, None));
    let mut profile = sample_profile(&key, None);
    profile.title = "Private Macro".to_string();
    profile.avatar_handle = Some(ImageHandle::from_bytes(vec![0x89, b'P', b'N', b'G']));

    let _task = terminal.store_telegram_channel_profile(profile);

    let profile = terminal
        .telegram_feed
        .channel_profiles
        .get(&key)
        .expect("private profile should be stored");
    assert_eq!(profile.title, "Private Macro");
    assert!(profile.avatar_handle.is_some());
}

#[test]
fn unselected_private_channel_fast_event_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let key = crate::telegram_feed::telegram_private_channel_key(42);
    let page = sample_page(&key, vec![sample_post(&key, 7)]);

    deliver_enabled_fast_feed_event(
        &mut terminal,
        TelegramFastFeedEvent::Loaded(key, Box::new(Ok(page))),
    );

    assert!(terminal.telegram_feed.posts.is_empty());
}

#[tokio::test]
async fn removing_private_channel_clears_posts_profile_cursor_and_reconnects() {
    let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let key = crate::telegram_feed::telegram_private_channel_key(42);
    set_fast_channel_cursor_for_test(&key, 99).await;
    assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 99);
    terminal.telegram_feed.private_channels =
        vec![crate::telegram_feed::TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Macro".to_string(),
        }];
    terminal.telegram_feed.posts = vec![sample_post(&key, 7)];
    terminal
        .telegram_feed
        .channel_profiles
        .insert(key.clone(), sample_profile(&key, None));
    let nonce = terminal.telegram_feed.fast_reconnect_nonce;

    let _task =
        terminal.update_telegram_feed(Message::TelegramFeedRemoveChannel(key.clone().into()));

    assert!(terminal.telegram_feed.private_channels.is_empty());
    assert!(terminal.telegram_feed.posts.is_empty());
    assert!(!terminal.telegram_feed.channel_profiles.contains_key(&key));
    assert_eq!(
        terminal.telegram_feed.fast_reconnect_nonce,
        nonce.saturating_add(1)
    );
    assert_eq!(fast_channel_cursor_message_id_for_test(&key).await, 0);
}

#[test]
fn stale_avatar_result_is_ignored() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let current_url = "https://example.com/current.jpg";
    let stale_url = "https://example.com/stale.jpg";
    let mut profile = sample_profile("marketfeed", Some(current_url));
    profile.avatar_loading_url = Some(current_url.to_string());
    profile.avatar_request_id = 2;
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal
        .telegram_feed
        .channel_profiles
        .insert("marketfeed".to_string(), profile);

    let _task = terminal.update_telegram_feed(Message::TelegramAvatarLoaded(
        "marketfeed".to_string(),
        stale_url.to_string(),
        1,
        Box::new(Ok(vec![0xFF, 0xD8, 0xFF])),
    ));

    let profile = terminal
        .telegram_feed
        .channel_profiles
        .get("marketfeed")
        .expect("profile should remain");
    assert!(profile.avatar_handle.is_none());
    assert_eq!(profile.avatar_loading_url.as_deref(), Some(current_url));
    assert_eq!(profile.avatar_request_id, 2);
}

#[test]
fn avatar_failure_sets_backoff_and_suppresses_immediate_refetch() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let avatar_url = "https://example.com/avatar.jpg";
    let mut profile = sample_profile("marketfeed", Some(avatar_url));
    profile.avatar_loading_url = Some(avatar_url.to_string());
    profile.avatar_request_id = 1;
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal
        .telegram_feed
        .channel_profiles
        .insert("marketfeed".to_string(), profile);

    let _task = terminal.update_telegram_feed(Message::TelegramAvatarLoaded(
        "marketfeed".to_string(),
        avatar_url.to_string(),
        1,
        Box::new(Err("avatar failed".to_string())),
    ));

    let failed_at_ms = terminal
        .telegram_feed
        .channel_profiles
        .get("marketfeed")
        .and_then(|profile| profile.avatar_failed_at_ms)
        .expect("failure timestamp should be stored");

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page_with_avatar(
            "marketfeed",
            avatar_url,
            vec![sample_post("marketfeed", 1)],
        )),
    );

    let profile = terminal
        .telegram_feed
        .channel_profiles
        .get("marketfeed")
        .expect("profile should remain");
    assert_eq!(profile.avatar_failed_at_ms, Some(failed_at_ms));
    assert!(profile.avatar_loading_url.is_none());
    assert!(profile.avatar_handle.is_none());
}

#[test]
fn adding_channel_below_public_limit_is_allowed() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS - 1)
        .map(|index| format!("channel_{index}"))
        .collect();
    terminal.telegram_feed.channel_input = "another_channel".to_string();

    let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

    assert!(
        terminal
            .telegram_feed
            .channels
            .contains(&"another_channel".to_string())
    );
    assert_eq!(
        terminal.telegram_feed.channels.len(),
        TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
    );
    assert!(
        terminal
            .telegram_feed
            .loading_channels
            .contains(&"another_channel".to_string())
    );
    assert_eq!(terminal.telegram_feed.last_error, None);
}

#[test]
fn adding_channel_at_public_limit_is_rejected() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS)
        .map(|index| format!("channel_{index}"))
        .collect();
    terminal.telegram_feed.channel_input = "another_channel".to_string();

    let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

    assert!(
        !terminal
            .telegram_feed
            .channels
            .contains(&"another_channel".to_string())
    );
    assert_eq!(
        terminal.telegram_feed.channels.len(),
        TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
    );
    assert!(
        !terminal
            .telegram_feed
            .loading_channels
            .contains(&"another_channel".to_string())
    );
    assert_eq!(
        terminal.telegram_feed.last_error,
        Some(format!(
            "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels"
        ))
    );
}

#[test]
fn refresh_caps_overfilled_public_channel_state() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = (0..TELEGRAM_FEED_MAX_PUBLIC_CHANNELS + 4)
        .map(|index| format!("channel_{index}"))
        .collect();

    let _task = terminal.request_telegram_feed_refresh();

    assert_eq!(
        terminal.telegram_feed.channels.len(),
        TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
    );
    assert_eq!(
        terminal.telegram_feed.loading_channels.len(),
        TELEGRAM_FEED_MAX_PUBLIC_CHANNELS
    );
    assert!(
        !terminal
            .telegram_feed
            .channels
            .contains(&format!("channel_{TELEGRAM_FEED_MAX_PUBLIC_CHANNELS}"))
    );
    assert_eq!(
        terminal.telegram_feed.last_error,
        Some(format!(
            "Telegram Feed supports up to {TELEGRAM_FEED_MAX_PUBLIC_CHANNELS} public channels; extra channels were ignored"
        ))
    );
}

#[tokio::test]
async fn adding_public_channel_clears_existing_fast_cursor() {
    let _cursor_guard = fast_channel_cursor_test_lock().lock().await;
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    set_fast_channel_cursor_for_test("another_channel", 99).await;
    assert_eq!(
        fast_channel_cursor_message_id_for_test("another_channel").await,
        99
    );
    terminal.telegram_feed.channels.clear();
    terminal.telegram_feed.channel_input = "another_channel".to_string();

    let _task = terminal.update_telegram_feed(Message::TelegramFeedAddChannel);

    assert_eq!(
        terminal.telegram_feed.channels,
        vec!["another_channel".to_string()]
    );
    assert_eq!(
        fast_channel_cursor_message_id_for_test("another_channel").await,
        0
    );
}

#[test]
fn initial_load_is_quiet_and_later_new_posts_alert() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.notifications_enabled = true;

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page(
            "marketfeed",
            vec![sample_post("marketfeed", 1)],
        )),
    );
    assert_eq!(terminal.telegram_feed.posts[0].first_seen_ms, 0);
    assert!(terminal.toasts.is_empty());

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page(
            "marketfeed",
            vec![sample_post("marketfeed", 2), sample_post("marketfeed", 1)],
        )),
    );

    let new_post = terminal
        .telegram_feed
        .posts
        .iter()
        .find(|post| post.message_id == 2)
        .expect("new post should be inserted");
    assert!(new_post.first_seen_ms > 0);
    assert_eq!(terminal.toasts.len(), 1);
    assert!(terminal.toasts[0].message.contains("@marketfeed"));
}

#[test]
fn hard_refresh_does_not_alert_for_seen_post_pruned_from_rendered_feed() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
    terminal.telegram_feed.notifications_enabled = true;
    let initial_posts = (1..=(crate::telegram_feed::TELEGRAM_FEED_RENDER_LIMIT as u64 + 1))
        .map(|message_id| sample_post("marketfeed", message_id))
        .collect();

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page("marketfeed", initial_posts)),
    );

    assert!(terminal.toasts.is_empty());
    assert!(
        !terminal
            .telegram_feed
            .posts
            .iter()
            .any(|post| post.message_id == 1)
    );

    load_public_feed(
        &mut terminal,
        "marketfeed",
        Ok(sample_page(
            "marketfeed",
            vec![sample_post("marketfeed", 1)],
        )),
    );

    assert!(terminal.toasts.is_empty());
}

#[test]
fn onboarding_dismiss_reaches_feed_and_persists_flag() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    assert_eq!(
        terminal.telegram_feed.current_screen(),
        crate::telegram_feed::TelegramFeedScreen::Connect
    );

    let _ = terminal.update_telegram_feed(Message::TelegramFeedDismissOnboarding);

    assert!(terminal.telegram_feed.onboarding_dismissed);
    assert_eq!(
        terminal.telegram_feed.current_screen(),
        crate::telegram_feed::TelegramFeedScreen::LiveFeed
    );
}

#[test]
fn connect_then_back_returns_to_onboarding_and_disables_fast() {
    // Toggling Fast Mode mutates the global pending-auth registry; serialize
    // with the other fast-auth tests.
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

    // "Connect Telegram" enables Fast Mode and shows the phone step.
    let _ = terminal.update_telegram_feed(Message::ToggleTelegramFastFeed);
    assert!(terminal.telegram_feed.fast_mode_enabled);
    assert_eq!(
        terminal.telegram_feed.current_screen(),
        crate::telegram_feed::TelegramFeedScreen::SignInPhone
    );

    // The back chevron tears Fast Mode down and returns to onboarding.
    let _ = terminal.update_telegram_feed(Message::TelegramFeedShowOnboarding);
    assert!(!terminal.telegram_feed.fast_mode_enabled);
    assert!(!terminal.telegram_feed.onboarding_dismissed);
    assert_eq!(
        terminal.telegram_feed.current_screen(),
        crate::telegram_feed::TelegramFeedScreen::Connect
    );
}

#[test]
fn telegram_code_input_keeps_only_five_digits() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());

    let _ = terminal.update_telegram_feed(Message::TelegramFastCodeChanged("12 34 567".into()));

    assert_eq!(terminal.telegram_feed.fast_code_input.as_str(), "12345");
}

#[test]
fn telegram_channel_list_expansion_is_runtime_only() {
    let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    assert!(!terminal.telegram_feed.channels_expanded);

    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFeedChannelsExpanded);
    assert!(terminal.telegram_feed.channels_expanded);

    let _task = terminal.update_telegram_feed(Message::ToggleTelegramFeedChannelsExpanded);
    assert!(!terminal.telegram_feed.channels_expanded);
}
