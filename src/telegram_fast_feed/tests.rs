use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::oneshot;

fn placeholder_pending_auths(entries: &[(&str, u64)]) -> HashMap<PendingAuthKey, PendingAuth> {
    entries
        .iter()
        .map(|(path, request_id)| ((PathBuf::from(path), *request_id), PendingAuth::Placeholder))
        .collect()
}

#[test]
fn connected_session_resets_reconnect_backoff() {
    let grown = Duration::from_secs(32);

    assert_eq!(
        fast_retry_delay_after_session(grown, true),
        TELEGRAM_FAST_RECONNECT_BASE_DELAY
    );
    assert_eq!(fast_retry_delay_after_session(grown, false), grown);
    assert_eq!(
        next_fast_reconnect_delay(TELEGRAM_FAST_RECONNECT_MAX_DELAY),
        TELEGRAM_FAST_RECONNECT_MAX_DELAY
    );
}

#[test]
fn clear_pending_auth_drops_abandoned_challenges() {
    let mut pending = placeholder_pending_auths(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-b.session", 2),
    ]);

    assert_eq!(clear_pending_auth_map(&mut pending), 2);
    assert_eq!(clear_pending_auth_map(&mut pending), 0);
}

#[test]
fn clear_pending_auth_for_request_drops_only_matching_challenges() {
    let mut pending = placeholder_pending_auths(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-b.session", 2),
    ]);

    assert_eq!(clear_pending_auth_map_for_request(&mut pending, 1), 1);
    assert!(pending.contains_key(&(PathBuf::from("/tmp/kerosene-telegram-b.session"), 2)));
    assert_eq!(clear_pending_auth_map(&mut pending), 1);
}

#[test]
fn clear_pending_auth_except_request_drops_abandoned_challenges() {
    let mut pending = placeholder_pending_auths(&[
        ("/tmp/kerosene-telegram-a.session", 1),
        ("/tmp/kerosene-telegram-a.session", 2),
        ("/tmp/kerosene-telegram-b.session", 3),
    ]);

    assert_eq!(clear_pending_auth_map_except_request(&mut pending, 2), 2);
    assert_eq!(pending.len(), 1);
    assert!(pending.contains_key(&(PathBuf::from("/tmp/kerosene-telegram-a.session"), 2)));
    assert_eq!(clear_pending_auth_map_except_request(&mut pending, 2), 0);
}

#[test]
fn sign_out_outcome_fails_when_local_session_clear_fails() {
    let result = telegram_fast_sign_out_outcome(
        Ok(()),
        Err("remove <config-dir>/telegram_fast.session failed: denied".to_string()),
    );

    let error = result.expect_err("local session clear failure should fail sign-out");
    assert!(error.starts_with(TELEGRAM_FAST_SESSION_CLEAR_FAILED));
    assert!(error.contains("denied"));
}

#[test]
fn session_file_error_display_redacts_parent_path() {
    let rendered = redacted_session_file_display(Path::new(
        "/home/alice/.config/kerosene/telegram_fast.session-wal",
    ));

    assert_eq!(rendered, "<config-dir>/telegram_fast.session-wal");
    assert!(!rendered.contains("/home/alice"));
}

#[test]
fn sign_out_outcome_warns_when_remote_sign_out_fails_but_local_session_clears() {
    let result = telegram_fast_sign_out_outcome(Err("network unavailable".to_string()), Ok(1))
        .expect("local session clear should complete sign-out");

    assert_eq!(
        result,
        TelegramFastAuthOutcome::SignedOut {
            warning: Some(TELEGRAM_FAST_REMOTE_SIGN_OUT_UNCONFIRMED.to_string())
        }
    );
}

#[test]
fn fast_updates_are_configured_live_first() {
    let config = live_first_updates_configuration();

    assert!(!config.catch_up);
    assert_eq!(
        config.update_queue_limit,
        Some(TELEGRAM_FAST_UPDATE_QUEUE_LIMIT)
    );
}

#[tokio::test]
async fn telegram_pool_shutdown_returns_without_abort_when_task_completes() {
    let task = tokio::spawn(async {});

    let aborted = shutdown_telegram_pool_task(task, Duration::from_secs(1)).await;

    assert!(!aborted);
}

#[tokio::test]
async fn telegram_pool_shutdown_aborts_after_timeout() {
    let (started_tx, started_rx) = oneshot::channel();
    let aborted = Arc::new(AtomicBool::new(false));
    let aborted_for_task = Arc::clone(&aborted);
    let task = tokio::spawn(async move {
        let _guard = DropGuard::new(move || {
            aborted_for_task.store(true, Ordering::SeqCst);
        });
        let _ = started_tx.send(());
        futures::future::pending::<()>().await;
    });
    started_rx.await.expect("task should start");

    let did_abort = shutdown_telegram_pool_task(task, Duration::from_millis(1)).await;

    assert!(did_abort);
    assert!(aborted.load(Ordering::SeqCst));
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

#[test]
fn fast_feed_stream_params_debug_redacts_private_channels() {
    let params = TelegramFastFeedStreamParams {
        api_id: 12345,
        channels: vec!["marketfeed".to_string()],
        private_channels: vec![TelegramFeedPrivateChannelConfig {
            peer_id: 42,
            title: "Private Macro".to_string(),
        }],
        reconnect_nonce: 7,
    };

    let rendered = format!("{params:?}");

    assert!(rendered.contains("marketfeed"));
    assert!(rendered.contains("api_id: \"<redacted>\""));
    assert!(rendered.contains("private_channels: <1 redacted>"));
    assert!(rendered.contains("reconnect_nonce: 7"));
    assert!(!rendered.contains("12345"));
    assert!(!rendered.contains("42"));
    assert!(!rendered.contains("Private Macro"));
}

#[test]
fn fast_channel_target_debug_redacts_private_identity_and_peer_ref() {
    let identity = FastChannelIdentity {
        key: "private:42".to_string(),
        title: "Private Macro".to_string(),
        cursor_generation: FastCursorGeneration {
            global: 1,
            channel: 2,
        },
    };
    let target = FastChannelTarget {
        profile: telegram_channel_profile_from_title(&identity.key, Some(&identity.title)),
        identity,
        peer_ref: PeerRef {
            id: PeerId::channel_unchecked(42),
            auth: grammers_session::types::PeerAuth::from_hash(98765),
        },
    };

    let rendered = format!("{target:?}");

    assert!(rendered.contains("<private>"));
    assert!(rendered.contains("peer_ref"));
    for secret in ["private:42", "Private Macro", "98765"] {
        assert!(!rendered.contains(secret), "debug leaked {secret}");
    }
}

#[test]
fn private_live_identity_resolves_from_peer_id_without_cached_peer() {
    let channels = vec![TelegramFeedPrivateChannelConfig {
        peer_id: 42,
        title: "Private Macro".to_string(),
    }];
    let mapped = normalized_private_channel_map(&channels);
    let generations = fast_cursor_generations_for_channels(&HashSet::new(), &mapped);

    let identity =
        private_identity_for_peer_id(PeerId::channel_unchecked(42), &mapped, &generations).unwrap();

    assert_eq!(identity.key, "private:42");
    assert_eq!(identity.title, "Private Macro");
    assert!(
        private_identity_for_peer_id(PeerId::chat_unchecked(42), &mapped, &generations).is_none()
    );
}

#[tokio::test]
async fn channel_cursor_clear_invalidates_late_records_from_old_generation() {
    let _guard = fast_channel_cursor_test_lock().lock().await;
    clear_all_fast_channel_cursors().await;
    let cursors = fast_channel_cursors();
    let channel = "marketfeed_cursor_clear";
    let initial_generation = fast_cursor_generation(channel);

    record_channel_cursor(&cursors, channel, 10, initial_generation).await;
    assert_eq!(
        channel_cursor_message_id(&cursors, channel, initial_generation).await,
        10
    );

    clear_fast_channel_cursor(channel);
    record_channel_cursor(&cursors, channel, 11, initial_generation).await;
    let next_generation = fast_cursor_generation(channel);

    assert_eq!(
        channel_cursor_message_id(&cursors, channel, initial_generation).await,
        0
    );
    assert_eq!(
        channel_cursor_message_id(&cursors, channel, next_generation).await,
        0
    );

    record_channel_cursor(&cursors, channel, 12, next_generation).await;
    assert_eq!(
        channel_cursor_message_id(&cursors, channel, next_generation).await,
        12
    );
    clear_all_fast_channel_cursors().await;
}

#[tokio::test]
async fn clearing_all_cursors_invalidates_late_records_from_old_generation() {
    let _guard = fast_channel_cursor_test_lock().lock().await;
    clear_all_fast_channel_cursors().await;
    let cursors = fast_channel_cursors();
    let channel = "marketfeed_clear_all";
    let initial_generation = fast_cursor_generation(channel);

    record_channel_cursor(&cursors, channel, 10, initial_generation).await;
    assert_eq!(
        channel_cursor_message_id(&cursors, channel, initial_generation).await,
        10
    );

    clear_all_fast_channel_cursors_best_effort();
    record_channel_cursor(&cursors, channel, 11, initial_generation).await;
    let next_generation = fast_cursor_generation(channel);

    assert_eq!(
        channel_cursor_message_id(&cursors, channel, initial_generation).await,
        0
    );
    assert_eq!(
        channel_cursor_message_id(&cursors, channel, next_generation).await,
        0
    );
    clear_all_fast_channel_cursors().await;
}
