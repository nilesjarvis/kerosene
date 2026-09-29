use super::*;
use crate::config::{CredentialStorageMode, KeroseneConfig, decrypt_secrets};
use crate::x_feed::{XFeedPost, XFeedState, XListOwnerKind, XListSummary};

fn test_user() -> XAuthenticatedUser {
    XAuthenticatedUser {
        id: "42".to_string(),
        username: "alice".to_string(),
        name: "Alice".to_string(),
    }
}

fn terminal_with_x_credentials(access: &str, client: &str, refresh: &str) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    terminal.x_feed = XFeedState::new(&[], access, client, refresh);
    terminal.secret_storage_mode = CredentialStorageMode::EncryptedConfig;
    terminal
}

#[test]
fn refresh_entrypoints_share_token_admission_and_suppress_pending_refreshes() {
    let refreshers: [fn(&mut TradingTerminal) -> Task<Message>; 3] = [
        TradingTerminal::request_x_feed_auth_refresh,
        TradingTerminal::request_x_feed_lists_refresh,
        |terminal| terminal.request_x_feed_refresh(17, true),
    ];
    for request_refresh in refreshers {
        for (access, client, refresh, expires_at, should_refresh) in [
            ("access", "client", "refresh", None, true),
            ("access", "client", "refresh", Some(0), true),
            ("access", "client", "refresh", Some(u64::MAX), false),
            ("", "client", "refresh", Some(u64::MAX), true),
            ("access", "", "refresh", None, false),
            ("access", "client", "", None, false),
            ("", "client", "", None, false),
            ("", "", "refresh", None, false),
        ] {
            let mut terminal = terminal_with_x_credentials(access, client, refresh);
            terminal
                .x_feed
                .set_oauth_credentials_from_secret(access, client, refresh, expires_at);
            terminal.x_feed.auth_user = Some(test_user());
            terminal
                .x_feed
                .instances
                .insert(17, XFeedInstance::new(17, XFeedSource::Following));

            let task = request_refresh(&mut terminal);

            assert_eq!(terminal.x_feed.token_refreshing, should_refresh);
            assert_eq!(
                terminal.x_feed.token_refresh_request_id,
                u64::from(should_refresh)
            );
            assert_eq!(
                task.units(),
                usize::from(should_refresh || !access.is_empty())
            );
            if should_refresh {
                assert_eq!(request_refresh(&mut terminal).units(), 0);
                assert_eq!(terminal.x_feed.token_refresh_request_id, 1);
                assert!(!terminal.x_feed.connecting);
                assert!(!terminal.x_feed.lists_loading);
                assert!(
                    !terminal
                        .x_feed
                        .source_refresh_in_flight(&XFeedSource::Following)
                );
            }
        }
    }
}

#[test]
fn feed_auth_fallback_clears_only_visible_errors_and_preserves_pending_connect() {
    for visible in [false, true] {
        for connecting in [false, true] {
            let mut terminal = terminal_with_x_credentials("access", "", "");
            terminal.x_feed.connecting = connecting;
            let mut instance = XFeedInstance::new(17, XFeedSource::Following);
            instance.last_error = Some("prior error".to_string());
            terminal.x_feed.instances.insert(17, instance);

            let task = terminal.request_x_feed_refresh(17, visible);

            assert_eq!(task.units(), usize::from(!connecting));
            assert!(terminal.x_feed.connecting);
            assert_eq!(terminal.x_feed.connect_request_id, u64::from(!connecting));
            assert_eq!(
                terminal.x_feed.instances[&17].last_error.as_deref(),
                if visible { None } else { Some("prior error") }
            );
        }
    }
}

#[test]
fn open_refresh_batches_each_source_once_across_canvas_instances() {
    let mut terminal = terminal_with_x_credentials("access", "", "");
    terminal.x_feed.auth_user = Some(test_user());
    let list = XFeedSource::List {
        id: "10".to_string(),
        name: "Markets".to_string(),
        private: false,
    };
    for (pane_id, id, source) in [
        (7, 17, XFeedSource::Following),
        (8, 18, XFeedSource::Following),
        (9, 19, list.clone()),
    ] {
        terminal
            .x_feed
            .instances
            .insert(id, XFeedInstance::new(id, source));
        terminal.insert_test_canvas_pane(pane_id, PaneKind::XFeed(id));
    }

    assert_eq!(terminal.request_x_feed_open_refresh(true).units(), 2);
    assert!(
        terminal
            .x_feed
            .source_refresh_in_flight(&XFeedSource::Following)
    );
    assert!(terminal.x_feed.source_refresh_in_flight(&list));
    assert_eq!(terminal.x_feed.refresh_request_id, 2);
    assert_eq!(terminal.request_x_feed_open_refresh(false).units(), 0);
    assert_eq!(terminal.x_feed.refresh_request_id, 2);
}

#[test]
fn refreshed_tokens_persist_rotated_or_fallback_credentials_before_authentication() {
    for pending in [false, true] {
        for rotated in [false, true] {
            let mut terminal =
                terminal_with_x_credentials("saved-access", "saved-client", "saved-refresh");
            terminal.encrypted_secret_password = "test-password".into();
            if pending {
                terminal.x_feed.oauth_client_id_input = "pending-client".into();
                terminal.x_feed.refresh_token_input = "pending-refresh".into();
                assert!(
                    terminal
                        .x_feed
                        .refresh_credentials_candidate_from_input()
                        .is_some()
                );
            }
            let request_id = terminal.x_feed.next_token_refresh_request_id();
            terminal.x_feed.token_refreshing = true;
            let result = XOAuthTokenRefresh {
                access_token: zeroize::Zeroizing::new("new-access".to_string()),
                refresh_token: rotated
                    .then(|| zeroize::Zeroizing::new("rotated-refresh".to_string())),
                expires_in_secs: Some(3_600),
            };

            assert_eq!(
                terminal
                    .handle_x_access_token_refreshed(request_id, Ok(result))
                    .units(),
                1
            );

            let expected_client = if pending {
                "pending-client"
            } else {
                "saved-client"
            };
            let expected_refresh = if rotated {
                "rotated-refresh"
            } else if pending {
                "pending-refresh"
            } else {
                "saved-refresh"
            };
            let (access, client, refresh) = terminal.x_feed.oauth_credentials_for_secret();
            assert_eq!(access.as_str(), "new-access");
            assert_eq!(client.as_str(), expected_client);
            assert_eq!(refresh.as_str(), expected_refresh);
            let persisted = decrypt_secrets(
                terminal
                    .encrypted_secrets
                    .as_ref()
                    .expect("persisted encrypted credentials"),
                "test-password",
            )
            .expect("test credentials decrypt");
            assert_eq!(persisted.global_x_access_token(), "new-access");
            assert_eq!(persisted.global_x_oauth_client_id(), expected_client);
            assert_eq!(persisted.global_x_refresh_token(), expected_refresh);
            assert!(
                terminal
                    .x_feed
                    .pending_oauth_credentials_for_secret()
                    .is_none()
            );
            assert!(!terminal.x_feed.token_refreshing);
            assert!(terminal.x_feed.connecting);
        }
    }
}

#[test]
fn stale_refresh_results_and_failed_saves_preserve_saved_credentials() {
    let mut terminal = terminal_with_x_credentials("saved-access", "saved-client", "saved-refresh");
    terminal.x_feed.oauth_client_id_input = "pending-client".into();
    terminal.x_feed.refresh_token_input = "pending-refresh".into();
    assert!(
        terminal
            .x_feed
            .refresh_credentials_candidate_from_input()
            .is_some()
    );
    let request_id = terminal.x_feed.next_token_refresh_request_id();
    terminal.x_feed.token_refreshing = true;
    let result = XOAuthTokenRefresh {
        access_token: zeroize::Zeroizing::new("new-access".to_string()),
        refresh_token: None,
        expires_in_secs: None,
    };
    assert_eq!(
        terminal
            .handle_x_access_token_refreshed(request_id - 1, Ok(result.clone()))
            .units(),
        0
    );
    assert_eq!(
        terminal
            .handle_x_access_token_refreshed(request_id - 1, Err("stale error".to_string()))
            .units(),
        0
    );
    assert!(terminal.x_feed.token_refreshing);
    assert!(
        terminal
            .x_feed
            .pending_oauth_credentials_for_secret()
            .is_some()
    );
    assert!(terminal.x_feed.status.is_none());

    // No encryption password: the current result cannot be committed.
    assert_eq!(
        terminal
            .handle_x_access_token_refreshed(request_id, Ok(result))
            .units(),
        0
    );
    let (access, client, refresh) = terminal.x_feed.oauth_credentials_for_secret();
    assert_eq!(access.as_str(), "saved-access");
    assert_eq!(client.as_str(), "saved-client");
    assert_eq!(refresh.as_str(), "saved-refresh");
    assert!(
        terminal
            .x_feed
            .pending_oauth_credentials_for_secret()
            .is_none()
    );
    assert!(!terminal.x_feed.token_refreshing);
    assert!(!terminal.x_feed.connecting);
    assert!(terminal.encrypted_secrets.is_none());
    assert_eq!(terminal.x_feed.status, terminal.secret_store_status);
    assert!(
        terminal
            .x_feed
            .status
            .as_ref()
            .is_some_and(|(_, error)| *error)
    );
}

#[test]
fn authentication_result_retains_user_lists_and_partial_source_status() {
    let mut terminal = terminal_with_x_credentials("access", "", "");
    let user = test_user();
    let lists = vec![XListSummary {
        id: "10".to_string(),
        name: "Markets".to_string(),
        private: false,
        owner: XListOwnerKind::Owned,
    }];
    let request_id = terminal.x_feed.next_connect_request_id();
    terminal.x_feed.connecting = true;

    assert_eq!(
        terminal
            .handle_x_feed_auth_loaded(
                request_id,
                Ok((
                    user.clone(),
                    XListsFetchOutcome {
                        lists: lists.clone(),
                        unavailable_sources: vec![XListOwnerKind::Followed]
                    }
                ))
            )
            .units(),
        0
    );

    assert_eq!(terminal.x_feed.auth_user, Some(user));
    assert_eq!(terminal.x_feed.lists, lists);
    assert!(!terminal.x_feed.connecting);
    assert_eq!(
        terminal.x_feed.status,
        Some((
            "Connected @alice; 1 Lists available; followed List source unavailable".to_string(),
            false
        ))
    );
}

fn test_post(author_id: &str, image_url: Option<&str>) -> XFeedPost {
    XFeedPost {
        id: "1".to_string(),
        author_id: Some(author_id.to_string()),
        author_name: "Alice Example".to_string(),
        author_username: "alice".to_string(),
        author_profile_image_url: image_url.map(str::to_string),
        text: "test post".to_string(),
        created_at_ms: 1_000,
        received_at_ms: 2_000,
        url: "https://example.com/post/1".to_string(),
    }
}

fn test_page(posts: Vec<XFeedPost>) -> XFeedPage {
    XFeedPage {
        source: XFeedSource::Following,
        posts,
        newest_id: None,
        rate_limited_until_ms: None,
    }
}

#[test]
fn open_refresh_includes_canvas_x_feed_instances() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let id = 17;
    terminal
        .x_feed
        .instances
        .insert(id, XFeedInstance::new(id, XFeedSource::Following));
    terminal.insert_test_canvas_pane(7, PaneKind::XFeed(id));

    let _task = terminal.request_x_feed_open_refresh(true);

    assert_eq!(
        terminal.x_feed.instances[&id].last_error.as_deref(),
        Some("Paste an X OAuth 2.0 user access token")
    );
}

#[test]
fn profile_fetches_dedupe_authors_and_update_metadata_without_an_image() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let original = test_post("42", Some("https://example.com/alice.jpg"));
    let mut renamed = original.clone();
    renamed.author_username = "alice_updated".to_string();
    renamed.author_name = "New Name".to_string();
    renamed.author_profile_image_url = None;
    let page = test_page(vec![
        original.clone(),
        original.clone(),
        renamed,
        test_post("43", original.author_profile_image_url.as_deref()),
        test_post("44", None),
    ]);

    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 2);

    let profile = &terminal.x_feed.author_profiles[&original.author_profile_key()];
    assert_eq!(profile.author_id.as_deref(), Some("42"));
    assert_eq!(profile.username, "alice_updated");
    assert_eq!(profile.name, "New Name");
    assert_eq!(profile.initials, "NN");
    assert_eq!(profile.profile_image_url, original.author_profile_image_url);
    assert_eq!(profile.image_loading_url, original.author_profile_image_url);
    assert_eq!(profile.image_request_id, 1);
    assert_eq!(terminal.x_feed.next_profile_image_request_id, 2);
    assert_eq!(terminal.x_feed.author_profiles.len(), 3);
    let no_image = &terminal.x_feed.author_profiles["id:44"];
    assert!(no_image.profile_image_url.is_none());
    assert!(no_image.image_loading_url.is_none());
    assert_eq!(no_image.image_request_id, 0);
}

#[test]
fn unchanged_or_missing_image_url_preserves_cached_pending_and_failed_profiles() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let original = test_post("42", Some("https://example.com/alice.jpg"));
    let key = original.author_profile_key();

    for state in 0..3 {
        let mut expected = XAuthorProfile::from_post(&original);
        match state {
            0 => expected.image_handle = Some(ImageHandle::from_rgba(1, 1, vec![0; 4])),
            1 => {
                expected.image_loading_url = original.author_profile_image_url.clone();
                expected.image_request_id = 7;
            }
            _ => expected.image_failed_at_ms = Some(u64::MAX),
        }
        terminal
            .x_feed
            .author_profiles
            .insert(key.clone(), expected.clone());
        for image_url in [original.author_profile_image_url.clone(), None] {
            let mut post = original.clone();
            post.author_profile_image_url = image_url;
            post.author_name = "Updated Name".to_string();
            expected.name.clone_from(&post.author_name);
            expected.initials = post.author_initials();

            assert_eq!(
                terminal
                    .schedule_x_profile_image_fetches(&test_page(vec![post]))
                    .units(),
                0
            );
            assert_eq!(terminal.x_feed.author_profiles[&key], expected);
        }
    }
    assert_eq!(terminal.x_feed.next_profile_image_request_id, 0);
}

#[test]
fn changed_image_rejects_stale_results_and_retries_after_backoff() {
    let (mut terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let mut post = test_post("42", Some("https://example.com/old.jpg"));
    let key = post.author_profile_key();
    assert_eq!(
        terminal
            .schedule_x_profile_image_fetches(&test_page(vec![post.clone()]))
            .units(),
        1
    );
    let old_request_id = terminal.x_feed.author_profiles[&key].image_request_id;
    post.author_profile_image_url = Some("https://example.com/new.jpg".to_string());
    let page = test_page(vec![post.clone()]);
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 1);
    let pending = terminal.x_feed.author_profiles[&key].clone();
    assert_ne!(pending.image_request_id, old_request_id);
    assert_eq!(pending.profile_image_url, post.author_profile_image_url);
    assert_eq!(pending.image_loading_url, post.author_profile_image_url);
    assert!(pending.image_handle.is_none());
    assert!(pending.image_failed_at_ms.is_none());

    terminal.handle_x_profile_image_loaded(old_request_id, Ok(vec![1, 2, 3]));
    terminal.handle_x_profile_image_loaded(old_request_id, Err("stale".to_string()));
    assert_eq!(terminal.x_feed.author_profiles[&key], pending);

    terminal.handle_x_profile_image_loaded(pending.image_request_id, Err("failed".to_string()));
    let failed = &terminal.x_feed.author_profiles[&key];
    assert!(failed.image_handle.is_none());
    assert!(failed.image_loading_url.is_none());
    assert_eq!(failed.image_request_id, 0);
    assert!(failed.image_failed_at_ms.is_some());
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 0);

    terminal
        .x_feed
        .author_profiles
        .get_mut(&key)
        .expect("existing profile")
        .image_failed_at_ms =
        Some(TradingTerminal::now_ms().saturating_sub(X_PROFILE_IMAGE_RETRY_BACKOFF_MS));
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 1);
    let retry = terminal.x_feed.author_profiles[&key].image_request_id;
    assert!(retry > pending.image_request_id);
    assert!(
        terminal.x_feed.author_profiles[&key]
            .image_failed_at_ms
            .is_none()
    );
    terminal.handle_x_profile_image_loaded(retry, Ok(vec![1, 2, 3]));
    let loaded = terminal.x_feed.author_profiles[&key].clone();
    assert!(loaded.image_handle.is_some());
    assert!(loaded.image_loading_url.is_none());
    assert_eq!(loaded.image_request_id, 0);
    assert!(loaded.image_failed_at_ms.is_none());
    assert_eq!(terminal.schedule_x_profile_image_fetches(&page).units(), 0);
    terminal.handle_x_profile_image_loaded(retry, Err("duplicate result".to_string()));
    assert_eq!(terminal.x_feed.author_profiles[&key], loaded);

    post.author_profile_image_url = Some("https://example.com/third.jpg".to_string());
    assert_eq!(
        terminal
            .schedule_x_profile_image_fetches(&test_page(vec![post]))
            .units(),
        1
    );
    assert!(terminal.x_feed.author_profiles[&key].image_handle.is_none());
}
