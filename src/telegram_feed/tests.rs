use super::*;

#[test]
fn available_private_candidates_preserve_scan_order_and_exclude_selected_peers() {
    let mut state = TelegramFeedState::new(&[], &[], false, true, Some(12345), true, false);
    state.private_channel_candidates = [42, 7, 9, 7]
        .into_iter()
        .map(|peer_id| TelegramPrivateChannelCandidate {
            peer_id,
            title: format!("Synthetic channel {peer_id}"),
            avatar_handle: None,
        })
        .collect();
    for (selected, expected) in [
        (vec![], vec![42, 7, 9, 7]),
        (vec![7], vec![42, 9]),
        (vec![7, 9, 42], vec![]),
        (vec![999], vec![42, 7, 9, 7]),
    ] {
        state.private_channels = selected
            .into_iter()
            .map(|peer_id| TelegramFeedPrivateChannelConfig {
                peer_id,
                title: "Selected channel".to_string(),
            })
            .collect();
        assert_eq!(
            state
                .available_private_channel_candidates()
                .iter()
                .map(|candidate| candidate.peer_id)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

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
