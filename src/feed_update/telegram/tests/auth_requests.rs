use super::*;

#[test]
fn auth_requests_preserve_challenge_state_and_advance_result_generation() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");

    for initial_id in [17, u64::MAX] {
        for (message, status, clears_code, clears_password, signs_out) in [
            (
                Message::TelegramFastRequestCode,
                "Requesting Telegram login code",
                false,
                false,
                false,
            ),
            (
                Message::TelegramFastSubmitCode,
                "Signing in to Telegram",
                true,
                false,
                false,
            ),
            (
                Message::TelegramFastSubmitPassword,
                "Checking Telegram 2FA password",
                false,
                true,
                false,
            ),
            (
                Message::TelegramFastSignOut,
                "Signing out of Telegram",
                false,
                false,
                true,
            ),
        ] {
            let (mut terminal, _boot_task) =
                TradingTerminal::boot_from_config(KeroseneConfig::default());
            let feed = &mut terminal.telegram_feed;
            feed.fast_api_id = Some(12345);
            feed.fast_api_id_input.clear();
            feed.fast_auth_request_id = initial_id;
            feed.fast_auth_stage = TelegramFastAuthStage::PasswordRequired;
            feed.fast_api_hash_input = "synthetic-hash".to_string().into();
            feed.fast_phone_input = "+15555550123".to_string();
            feed.fast_code_input = " 12345 ".to_string().into();
            feed.fast_password_input = " synthetic-password ".to_string().into();
            feed.fast_password_hint = Some("synthetic hint".to_string());
            feed.fast_code_sent_at_ms = Some(100);
            feed.fast_reconnect_nonce = 9;
            feed.private_channel_candidates_request_id = 20;
            feed.private_channel_candidates_loading = true;
            set_telegram_fast_pending_auth_placeholders_for_test(&[(
                "/tmp/kerosene-telegram-request-test.session",
                initial_id,
            )]);

            // Inspect scheduling only; these real request tasks are never polled.
            let task = terminal.update_telegram_feed(message);

            assert_eq!(task.units(), 1);
            let feed = &terminal.telegram_feed;
            assert_eq!(feed.fast_auth_request_id, initial_id.saturating_add(1));
            assert!(feed.fast_auth_in_flight);
            assert_eq!(feed.fast_status, Some((status.to_string(), false)));
            assert_eq!(
                feed.fast_auth_stage,
                TelegramFastAuthStage::PasswordRequired
            );
            assert!(!feed.fast_connected);
            assert_eq!(feed.fast_reconnect_nonce, 9);
            assert_eq!(feed.fast_code_sent_at_ms, Some(100));
            assert_eq!(feed.fast_password_hint.as_deref(), Some("synthetic hint"));
            assert_eq!(feed.fast_code_input.is_empty(), clears_code);
            assert_eq!(feed.fast_password_input.is_empty(), clears_password);
            assert!(!feed.fast_api_hash_input.is_empty());
            assert!(!feed.fast_phone_input.is_empty());
            assert_eq!(
                feed.private_channel_candidates_request_id,
                if signs_out { 21 } else { 20 }
            );
            assert_eq!(feed.private_channel_candidates_loading, !signs_out);
            assert_eq!(
                telegram_fast_pending_auth_request_ids_for_test(),
                if signs_out { vec![] } else { vec![initial_id] }
            );
        }
    }
    clear_telegram_fast_pending_auth();
}

#[test]
fn invalid_api_id_keeps_auth_inputs_and_request_state() {
    let _guard = telegram_fast_pending_auth_test_lock()
        .lock()
        .expect("pending auth test lock");

    for (message, signs_out) in [
        (Message::TelegramFastRequestCode, false),
        (Message::TelegramFastSubmitCode, false),
        (Message::TelegramFastSubmitPassword, false),
        (Message::TelegramFastSignOut, true),
    ] {
        let (mut terminal, _boot_task) =
            TradingTerminal::boot_from_config(KeroseneConfig::default());
        let feed = &mut terminal.telegram_feed;
        feed.fast_api_id_input = "-1".to_string();
        feed.fast_auth_request_id = 17;
        feed.fast_auth_stage = TelegramFastAuthStage::CodeRequested;
        feed.fast_code_input = "12345".to_string().into();
        feed.fast_password_input = "synthetic-password".to_string().into();
        feed.private_channel_candidates_request_id = 20;
        feed.private_channel_candidates_loading = true;
        set_telegram_fast_pending_auth_placeholders_for_test(&[(
            "/tmp/kerosene-telegram-request-test.session",
            17,
        )]);

        let task = terminal.update_telegram_feed(message);

        assert_eq!(task.units(), 0);
        let feed = &terminal.telegram_feed;
        assert_eq!(feed.fast_auth_request_id, 17);
        assert!(!feed.fast_auth_in_flight);
        assert_eq!(feed.fast_auth_stage, TelegramFastAuthStage::CodeRequested);
        assert!(!feed.fast_code_input.is_empty());
        assert!(!feed.fast_password_input.is_empty());
        assert_eq!(feed.private_channel_candidates_request_id, 20);
        assert!(feed.private_channel_candidates_loading);
        assert_eq!(
            feed.fast_status,
            Some((
                "Telegram API ID must be a positive number".to_string(),
                true
            ))
        );
        // Sign-out abandons the challenge even when API ID validation fails.
        assert_eq!(
            telegram_fast_pending_auth_request_ids_for_test(),
            if signs_out { vec![] } else { vec![17] }
        );
    }
    clear_telegram_fast_pending_auth();
}
