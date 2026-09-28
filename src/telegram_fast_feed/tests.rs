use super::*;

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
fn fast_updates_are_configured_live_first() {
    let config = live_first_updates_configuration();

    assert!(!config.catch_up);
    assert_eq!(
        config.update_queue_limit,
        Some(TELEGRAM_FAST_UPDATE_QUEUE_LIMIT)
    );
}

#[test]
fn private_channel_candidates_keep_title_order_and_first_adjacent_duplicate() {
    let mut candidates = [
        (4, "beta"),
        (7, "ALPHA"),
        (7, "alpha"),
        (3, "Alpha"),
        (9, "Älpha"),
        (8, "älpha"),
        (7, "Gamma"),
        (4, "BETA"),
        (6, "beta"),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (peer_id, title))| TelegramPrivateChannelCandidate {
            peer_id,
            title: title.to_string(),
            avatar_handle: Some(iced::widget::image::Handle::from_rgba(
                1,
                1,
                vec![index as u8, 0, 0, 255],
            )),
        },
    )
    .collect::<Vec<_>>();
    // Case-folded title ties use peer IDs, then retain input order. Deduplication
    // removes adjacent peer IDs only, so the later Gamma entry remains.
    let expected = [3, 1, 0, 8, 6, 4, 5].map(|index| candidates[index].clone());

    sort_and_dedup_private_channel_candidates(&mut candidates);

    assert_eq!(candidates, expected);
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
