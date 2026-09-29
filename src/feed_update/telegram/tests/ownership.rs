use super::*;

#[test]
fn alerts_preserve_arrival_order_limit_and_count_even_when_posts_are_pruned() {
    for enabled in [false, true] {
        let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.sound_enabled = false;
        terminal.desktop_notifications = false;
        terminal.telegram_feed.channels = vec!["marketfeed".to_string()];
        terminal.telegram_feed.notifications_enabled = enabled;
        let limit = crate::telegram_feed::TELEGRAM_FEED_RENDER_LIMIT;
        let initial = (1_000..1_000 + limit as u64)
            .map(|id| sample_post("marketfeed", id))
            .collect();
        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", initial)),
        );
        assert!(terminal.toasts.is_empty());
        terminal.telegram_feed.record_seen_post("marketfeed", 50);

        let mut posts: Vec<_> = [6, 2, 5, 1, 4, 3]
            .into_iter()
            .map(|id| {
                let mut post = sample_post("marketfeed", id);
                post.text = format!("headline {id}\nsecond line");
                post
            })
            .collect();
        let mut duplicate = posts[0].clone();
        duplicate.text = "edited headline".to_string();
        posts.push(duplicate);
        let mut backfill = sample_post("marketfeed", 7);
        backfill.source = TelegramFeedPostSource::FastBackfill;
        posts.extend([
            backfill,
            sample_post("marketfeed", 50),
            sample_post("marketfeed", 1_000),
        ]);
        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", posts.clone())),
        );

        let messages: Vec<_> = terminal
            .toasts
            .iter()
            .map(|toast| toast.message.as_str())
            .collect();
        if enabled {
            assert_eq!(
                messages,
                [
                    "@marketfeed: headline 6",
                    "@marketfeed: headline 2",
                    "@marketfeed: headline 5",
                    "3 more Telegram messages"
                ]
            );
        } else {
            assert!(messages.is_empty());
        }
        assert_eq!(terminal.telegram_feed.posts.len(), limit);
        assert!(
            terminal
                .telegram_feed
                .posts
                .iter()
                .all(|post| post.message_id >= 1_000)
        );
        terminal.toasts.clear();
        load_public_feed(
            &mut terminal,
            "marketfeed",
            Ok(sample_page("marketfeed", posts)),
        );
        assert!(
            terminal.toasts.is_empty(),
            "pruned posts are still remembered"
        );
    }
}

#[test]
fn media_scheduling_preserves_eligibility_order_and_retry_state() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let media = TelegramPostMedia::from_url(
        TelegramMediaKind::Photo,
        "https://example.com/photo.jpg".to_string(),
    );
    let mut loaded = media.clone();
    loaded.handle = Some(ImageHandle::from_bytes(vec![1]));
    let mut pending = media.clone();
    pending.loading_url = media.url.clone();
    pending.request_id = 9;
    let mut failed = media.clone();
    failed.failed_at_ms = Some(u64::MAX);
    let mut expired = media.clone();
    expired.failed_at_ms = Some(0);
    let mut changed = media.clone();
    changed.loading_url = Some("https://example.com/old.jpg".to_string());
    changed.request_id = 10;
    terminal.telegram_feed.posts = vec![
        sample_post_with_media("marketfeed", 1, loaded),
        sample_post_with_media("marketfeed", 2, pending),
        sample_post_with_media("marketfeed", 3, failed),
        sample_post_with_media("marketfeed", 4, expired),
        sample_post_with_media("marketfeed", 5, media.clone()),
        sample_post_with_media("marketfeed", 6, changed),
        sample_post_with_media(
            "marketfeed",
            7,
            TelegramPostMedia::placeholder(TelegramMediaKind::Photo),
        ),
        sample_post("marketfeed", 8),
        sample_post_with_media("otherfeed", 9, media.clone()),
    ];
    let before = terminal.telegram_feed.posts.clone();
    terminal.telegram_feed.next_media_request_id = 40;

    assert_eq!(
        terminal
            .schedule_telegram_media_fetches("marketfeed")
            .units(),
        3
    );

    for index in [0, 1, 2, 6, 7, 8] {
        assert_eq!(terminal.telegram_feed.posts[index], before[index]);
    }
    for (id, request_id) in [(4, 41), (5, 42), (6, 43)] {
        let pending = find_post_media(&terminal, "marketfeed", id);
        assert_eq!(pending.loading_url, media.url);
        assert_eq!(pending.request_id, request_id);
        assert!(pending.failed_at_ms.is_none());
    }
    assert_eq!(terminal.telegram_feed.next_media_request_id, 43);
    assert_eq!(
        terminal
            .schedule_telegram_media_fetches("marketfeed")
            .units(),
        0
    );
    assert_eq!(terminal.telegram_feed.next_media_request_id, 43);
}

#[test]
fn avatar_merging_preserves_unchanged_state_and_refetches_changed_urls() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let old_url = "https://example.com/old.jpg";
    let new_url = "https://example.com/new.jpg";
    for state in 0..3 {
        let mut existing = sample_profile("marketfeed", Some(old_url));
        match state {
            0 => existing.avatar_handle = Some(ImageHandle::from_bytes(vec![1])),
            1 => {
                existing.avatar_loading_url = Some(old_url.to_string());
                existing.avatar_request_id = 7;
            }
            _ => existing.avatar_failed_at_ms = Some(u64::MAX),
        }
        for url in [None, Some(old_url), Some(new_url)] {
            terminal
                .telegram_feed
                .channel_profiles
                .insert("marketfeed".to_string(), existing.clone());
            terminal.telegram_feed.next_avatar_request_id = 20;
            let mut incoming = sample_profile("marketfeed", url);
            incoming.title = "Updated title".to_string();
            incoming.initials = "UT".to_string();

            let task = terminal.store_telegram_channel_profile(incoming);

            let profile = &terminal.telegram_feed.channel_profiles["marketfeed"];
            assert_eq!(profile.title, "Updated title");
            assert_eq!(profile.initials, "UT");
            if url == Some(new_url) {
                assert_eq!(task.units(), 1);
                assert_eq!(profile.avatar_url.as_deref(), Some(new_url));
                assert_eq!(profile.avatar_loading_url.as_deref(), Some(new_url));
                assert_eq!(profile.avatar_request_id, 21);
                assert!(profile.avatar_handle.is_none());
                assert!(profile.avatar_failed_at_ms.is_none());
            } else {
                assert_eq!(task.units(), 0);
                assert_eq!(profile.avatar_url, existing.avatar_url);
                assert_eq!(profile.avatar_handle, existing.avatar_handle);
                assert_eq!(profile.avatar_loading_url, existing.avatar_loading_url);
                assert_eq!(profile.avatar_request_id, existing.avatar_request_id);
                assert_eq!(profile.avatar_failed_at_ms, existing.avatar_failed_at_ms);
                assert_eq!(terminal.telegram_feed.next_avatar_request_id, 20);
            }
        }
    }
}

#[test]
fn mention_refresh_replaces_match_metadata_but_preserves_reference_history() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.exchange_symbols = vec![exchange_symbol("BTC", "BTC")];
    terminal
        .telegram_feed
        .rebuild_ticker_mention_resolver(&terminal.exchange_symbols);
    let matched = terminal
        .telegram_feed
        .resolve_ticker_mentions("BTC")
        .remove(0);
    let previous = TelegramTickerMention {
        symbol: "BTC".to_string(),
        ticker: "old ticker".to_string(),
        matched_text: "old match".to_string(),
        source: crate::symbol_mentions::SymbolAliasSource::Keyword,
        confidence: 1,
        reference_price: Some(123.0),
        reference_seen_ms: 42,
    };
    for anchor in [false, true] {
        if anchor {
            terminal.record_screener_mid_samples(
                &std::collections::HashMap::from([("BTC".to_string(), 100.0)]),
                500,
            );
        }
        for price in [None, Some(123.0)] {
            let first = TelegramTickerMention {
                reference_price: price,
                ..previous.clone()
            };
            let duplicate = TelegramTickerMention {
                reference_price: Some(999.0),
                ..previous.clone()
            };
            let mentions = terminal.telegram_ticker_mentions_for_text(
                "BTC",
                1_000,
                60_000,
                &[first, duplicate],
            );
            let mention = &mentions[0];
            assert_eq!(mentions.len(), 1);
            assert_eq!(mention.symbol, "BTC");
            assert_eq!(mention.ticker, matched.ticker);
            assert_eq!(mention.matched_text, matched.matched_text);
            assert_eq!(mention.source, matched.source);
            assert_eq!(mention.confidence, matched.confidence);
            assert_eq!(
                mention.reference_price,
                price.or(if anchor { Some(100.0) } else { None })
            );
            assert_eq!(
                mention.reference_seen_ms,
                if price.is_none() && anchor {
                    60_000
                } else {
                    42
                }
            );
        }
    }
}
