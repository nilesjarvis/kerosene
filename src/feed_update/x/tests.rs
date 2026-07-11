use super::*;
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config;
use crate::x_feed::{
    XAuthenticatedUser, XFeedInstance, XFeedPage, XFeedPost, XFeedRequestError, XFeedSource,
    XListOwnerKind, XListSummary, XListsFetchOutcome, XOAuthTokenRefresh,
};

fn set_access_token_input(terminal: &mut TradingTerminal, token: &str) {
    let _task = terminal.update_x_feed(Message::XFeedAccessTokenChanged(token.into()));
}

fn set_refresh_credential_inputs(
    terminal: &mut TradingTerminal,
    client_id: &str,
    refresh_token: &str,
) {
    let _task = terminal.update_x_feed(Message::XFeedOAuthClientIdChanged(client_id.into()));
    let _task = terminal.update_x_feed(Message::XFeedRefreshTokenChanged(refresh_token.into()));
}

fn configure_empty_encrypted_secret_store(terminal: &mut TradingTerminal) {
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secret_password = sensitive_string("test-password");
    let payload = config::SecretPayload::from_credentials(&[], "hydro-key", "");
    terminal.encrypted_secrets = Some(
        config::encrypt_secrets(&payload, &terminal.encrypted_secret_password)
            .expect("test encrypted payload"),
    );
    terminal.encrypted_secrets_unlocked = true;
    terminal.hydromancer_api_key = sensitive_string("hydro-key");
    terminal.secret_migration_save_blocked = false;
    terminal.secret_store_status = None;
}

fn configure_x_runtime(terminal: &mut TradingTerminal, user_id: &str) {
    terminal
        .x_feed
        .set_oauth_credentials_from_secret("test-access-token", "", "", None);
    terminal.x_feed.auth_user = Some(XAuthenticatedUser {
        id: user_id.to_string(),
        username: format!("user-{user_id}"),
        name: "Test User".to_string(),
    });
}

fn test_post(id: &str, author_id: &str, image_url: Option<&str>, created_at_ms: u64) -> XFeedPost {
    XFeedPost {
        id: id.to_string(),
        author_id: Some(author_id.to_string()),
        author_name: format!("Author {author_id}"),
        author_username: format!("author-{author_id}"),
        author_profile_image_url: image_url.map(str::to_string),
        text: format!("post-{id}"),
        created_at_ms,
        received_at_ms: created_at_ms,
        url: format!("https://x.invalid/status/{id}"),
    }
}

fn test_page(
    source: XFeedSource,
    posts: Vec<XFeedPost>,
    rate_limited_until_ms: Option<u64>,
) -> XFeedPage {
    XFeedPage {
        source,
        newest_id: posts.last().map(|post| post.id.clone()),
        posts,
        rate_limited_until_ms,
    }
}

fn test_list(id: &str, name: &str) -> XListSummary {
    XListSummary {
        id: id.to_string(),
        name: name.to_string(),
        private: false,
        owner: XListOwnerKind::Owned,
    }
}

#[test]
fn in_flight_token_refresh_remains_owner_over_later_direct_auth() {
    let (mut terminal, _) = TradingTerminal::boot();
    set_refresh_credential_inputs(&mut terminal, "client-a", "refresh-a");
    let _older_task = terminal.update_x_feed(Message::XFeedConnect);
    let older_request_id = terminal
        .x_feed
        .current_token_refresh_request_id()
        .expect("token refresh owner");
    assert!(terminal.x_feed.token_refreshing);

    set_access_token_input(&mut terminal, "access-b");
    let _suppressed_task = terminal.update_x_feed(Message::XFeedConnect);
    assert!(!terminal.x_feed.connecting);
    assert!(terminal.x_feed.token_refreshing);
    assert_eq!(
        terminal.x_feed.current_token_refresh_request_id(),
        Some(older_request_id)
    );
    assert_eq!(
        terminal.x_feed.status,
        Some(("Refreshing X token".to_string(), false))
    );

    let _task = terminal.update_x_feed(Message::XAccessTokenRefreshed(
        older_request_id,
        XAccessTokenRefreshMessageResult::new(Err("older refresh failed".to_string())),
    ));

    assert!(!terminal.x_feed.connecting);
    assert!(!terminal.x_feed.token_refreshing);
    assert_eq!(
        terminal.x_feed.status,
        Some(("older refresh failed".to_string(), true))
    );
}

#[test]
fn newer_token_refresh_owns_status_over_older_direct_auth_result() {
    let (mut terminal, _) = TradingTerminal::boot();
    set_access_token_input(&mut terminal, "access-a");
    let _older_task = terminal.update_x_feed(Message::XFeedConnect);
    let older_request_id = terminal
        .x_feed
        .current_auth_request_id()
        .expect("auth owner");
    assert!(terminal.x_feed.connecting);

    set_refresh_credential_inputs(&mut terminal, "client-b", "refresh-b");
    let _newer_task = terminal.update_x_feed(Message::XFeedConnect);
    assert!(terminal.x_feed.token_refreshing);
    assert!(terminal.x_feed.current_lists_request_id().is_none());
    assert_eq!(
        terminal.x_feed.status,
        Some(("Refreshing X token".to_string(), false))
    );

    let _task = terminal.update_x_feed(Message::XFeedAuthLoaded(
        older_request_id,
        XAuthContextMessageResult::new(Err("older auth failed".to_string())),
    ));

    assert!(terminal.x_feed.token_refreshing);
    assert_eq!(
        terminal.x_feed.status,
        Some(("Refreshing X token".to_string(), false))
    );
}

#[test]
fn automatic_credential_work_does_not_supersede_explicit_in_flight_intent() {
    let (mut auth_terminal, _) = TradingTerminal::boot();
    auth_terminal.x_feed.set_oauth_credentials_from_secret(
        "stored-access",
        "stored-client",
        "stored-refresh",
        Some(u64::MAX),
    );
    set_access_token_input(&mut auth_terminal, "direct-access");
    let _auth_task = auth_terminal.update_x_feed(Message::XFeedConnect);
    let auth_request_id = auth_terminal
        .x_feed
        .current_auth_request_id()
        .expect("explicit auth owner");

    let _automatic_refresh = auth_terminal.request_x_access_token_refresh();

    assert_eq!(
        auth_terminal.x_feed.current_auth_request_id(),
        Some(auth_request_id)
    );
    assert!(!auth_terminal.x_feed.token_refreshing);

    let (mut refresh_terminal, _) = TradingTerminal::boot();
    refresh_terminal.x_feed.set_oauth_credentials_from_secret(
        "stored-access",
        "stored-client",
        "stored-refresh",
        Some(u64::MAX),
    );
    set_refresh_credential_inputs(&mut refresh_terminal, "new-client", "new-refresh");
    let _refresh_task = refresh_terminal.update_x_feed(Message::XFeedConnect);
    let refresh_request_id = refresh_terminal
        .x_feed
        .current_token_refresh_request_id()
        .expect("explicit token refresh owner");

    let _automatic_auth = refresh_terminal.request_x_feed_auth_refresh();

    assert_eq!(
        refresh_terminal.x_feed.current_token_refresh_request_id(),
        Some(refresh_request_id)
    );
    assert!(!refresh_terminal.x_feed.connecting);
}

#[test]
fn credential_handler_redacts_secrets_and_preserves_ordinary_errors() {
    let (mut terminal, _) = TradingTerminal::boot();
    set_access_token_input(&mut terminal, "direct-access");
    let _auth_task = terminal.update_x_feed(Message::XFeedConnect);
    let auth_request_id = terminal
        .x_feed
        .current_auth_request_id()
        .expect("auth owner");
    let ordinary_error = "X auth check returned HTTP 401".to_string();

    let _task = terminal.update_x_feed(Message::XFeedAuthLoaded(
        auth_request_id,
        XAuthContextMessageResult::new(Err(ordinary_error.clone())),
    ));

    assert_eq!(terminal.x_feed.status, Some((ordinary_error, true)));

    set_refresh_credential_inputs(&mut terminal, "client-a", "refresh-a");
    let _refresh_task = terminal.update_x_feed(Message::XFeedConnect);
    let refresh_request_id = terminal
        .x_feed
        .current_token_refresh_request_id()
        .expect("token refresh owner");
    let _task = terminal.update_x_feed(Message::XAccessTokenRefreshed(
        refresh_request_id,
        XAccessTokenRefreshMessageResult::new(Err(
            "X token refresh failed: auth_token=handler-secret".to_string(),
        )),
    ));

    let (message, is_error) = terminal.x_feed.status.as_ref().expect("error status");
    assert!(*is_error);
    assert!(message.contains("auth_token=<redacted>"), "{message}");
    assert!(!message.contains("handler-secret"), "{message}");
}

#[test]
fn accepted_direct_auth_preserves_commit_and_status_behavior() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_empty_encrypted_secret_store(&mut terminal);
    terminal.config_save_due_at = None;
    set_access_token_input(&mut terminal, "direct-access");
    let _auth_task = terminal.update_x_feed(Message::XFeedConnect);
    let auth_request_id = terminal
        .x_feed
        .current_auth_request_id()
        .expect("auth owner");

    let _task = terminal.update_x_feed(Message::XFeedAuthLoaded(
        auth_request_id,
        XAuthContextMessageResult::new(Ok((
            XAuthenticatedUser {
                id: "user-id".to_string(),
                username: "test-user".to_string(),
                name: "Test User".to_string(),
            },
            XListsFetchOutcome {
                lists: Vec::new(),
                unavailable_sources: Vec::new(),
            },
        ))),
    ));

    let (access_token, client_id, refresh_token) = terminal.x_feed.oauth_credentials_for_secret();
    assert_eq!(access_token.as_str(), "direct-access");
    assert_eq!(client_id.as_str(), "");
    assert_eq!(refresh_token.as_str(), "");
    assert!(!terminal.x_feed.connecting);
    assert!(terminal.config_save_due_at.is_some());
    assert_eq!(
        terminal.x_feed.status,
        Some(("Connected @test-user; 0 Lists available".to_string(), false,))
    );

    let accepted_status = terminal.x_feed.status.clone();
    let _replay = terminal.update_x_feed(Message::XFeedAuthLoaded(
        auth_request_id,
        XAuthContextMessageResult::new(Err("replayed auth error".to_string())),
    ));
    assert_eq!(terminal.x_feed.status, accepted_status);
}

#[test]
fn suppressed_refresh_retry_cannot_replace_active_request_credentials() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_empty_encrypted_secret_store(&mut terminal);
    set_refresh_credential_inputs(&mut terminal, "client-a", "refresh-a");
    let _older_task = terminal.update_x_feed(Message::XFeedConnect);
    let request_id = terminal
        .x_feed
        .current_token_refresh_request_id()
        .expect("token refresh owner");

    set_refresh_credential_inputs(&mut terminal, "client-b", "refresh-b");
    let _suppressed_task = terminal.update_x_feed(Message::XFeedConnect);
    assert_eq!(
        terminal.x_feed.current_token_refresh_request_id(),
        Some(request_id)
    );

    let _task = terminal.update_x_feed(Message::XAccessTokenRefreshed(
        request_id,
        XAccessTokenRefreshMessageResult::new(Ok(XOAuthTokenRefresh {
            access_token: "access-from-a".to_string().into(),
            refresh_token: None,
            expires_in_secs: Some(3_600),
        })),
    ));

    let (access_token, client_id, refresh_token) = terminal.x_feed.oauth_credentials_for_secret();
    assert_eq!(access_token.as_str(), "access-from-a");
    assert_eq!(client_id.as_str(), "client-a");
    assert_eq!(refresh_token.as_str(), "refresh-a");

    let accepted_status = terminal.x_feed.status.clone();
    let current_auth_request_id = terminal.x_feed.current_auth_request_id();
    let _replay = terminal.update_x_feed(Message::XAccessTokenRefreshed(
        request_id,
        XAccessTokenRefreshMessageResult::new(Err("replayed token refresh error".to_string())),
    ));
    assert_eq!(terminal.x_feed.status, accepted_status);
    assert_eq!(
        terminal.x_feed.current_auth_request_id(),
        current_auth_request_id
    );
}

#[test]
fn clear_and_runtime_reset_end_old_auth_without_reusing_its_owner() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_empty_encrypted_secret_store(&mut terminal);
    set_access_token_input(&mut terminal, "access-a");
    let _old_task = terminal.update_x_feed(Message::XFeedConnect);
    let old_request_id = terminal
        .x_feed
        .current_auth_request_id()
        .expect("auth owner");
    assert!(terminal.x_feed.connecting);

    let _clear_task = terminal.update_x_feed(Message::XFeedClearAccessToken);
    assert!(!terminal.x_feed.connecting);
    assert!(!terminal.x_feed.loading());
    assert_eq!(
        terminal.x_feed.status,
        Some(("X token cleared".to_string(), false))
    );

    let _reset_task = terminal.apply_config_clear_to_runtime(config::ClearConfigSummary {
        files_removed: 0,
        file_cleanup_failed: false,
        keychain_entries_cleared: 0,
        warnings: Vec::new(),
    });
    set_access_token_input(&mut terminal, "access-b");
    let _new_task = terminal.update_x_feed(Message::XFeedConnect);
    let new_request_id = terminal
        .x_feed
        .current_auth_request_id()
        .expect("post-clear auth owner");
    assert_ne!(new_request_id, old_request_id);
    assert_eq!(
        terminal.x_feed.status,
        Some(("Connecting to X".to_string(), false))
    );

    let _task = terminal.update_x_feed(Message::XFeedAuthLoaded(
        old_request_id,
        XAuthContextMessageResult::new(Err("old auth failed".to_string())),
    ));

    assert!(terminal.x_feed.connecting);
    assert_eq!(
        terminal.x_feed.status,
        Some(("Connecting to X".to_string(), false))
    );
}

#[test]
fn newest_lists_request_settles_once_and_preserves_success_behavior() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");

    let first_task = terminal.request_x_feed_lists_refresh();
    let first_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("first Lists owner");
    let second_task = terminal.request_x_feed_lists_refresh();
    let second_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("second Lists owner");

    assert_eq!(first_task.units(), 1);
    assert_eq!(second_task.units(), 1);
    assert_ne!(first_request_id, second_request_id);

    let _stale = terminal.update_x_feed(Message::XFeedListsLoaded(
        first_request_id,
        XListsMessageResult::new(Err("older Lists failure".to_string())),
    ));
    assert!(terminal.x_feed.lists_loading);
    assert_eq!(
        terminal.x_feed.status,
        Some(("Refreshing X Lists".to_string(), false))
    );

    let _accepted = terminal.update_x_feed(Message::XFeedListsLoaded(
        second_request_id,
        XListsMessageResult::new(Ok(XListsFetchOutcome {
            lists: vec![test_list("list-a", "List A")],
            unavailable_sources: vec![XListOwnerKind::Followed],
        })),
    ));
    assert!(!terminal.x_feed.lists_loading);
    assert_eq!(terminal.x_feed.lists, vec![test_list("list-a", "List A")]);
    assert_eq!(
        terminal.x_feed.status,
        Some((
            "Loaded 1 X Lists; followed List source unavailable".to_string(),
            false,
        ))
    );

    let accepted_lists = terminal.x_feed.lists.clone();
    let accepted_status = terminal.x_feed.status.clone();
    let _replay = terminal.update_x_feed(Message::XFeedListsLoaded(
        second_request_id,
        XListsMessageResult::new(Err("replayed Lists failure".to_string())),
    ));
    assert_eq!(terminal.x_feed.lists, accepted_lists);
    assert_eq!(terminal.x_feed.status, accepted_status);
}

#[test]
fn lists_owner_wraps_without_aliasing_and_redacts_only_sensitive_errors() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");
    terminal
        .x_feed
        .set_noncredential_request_allocators_for_test(u64::MAX - 1, 0, 0);

    let _first_task = terminal.request_x_feed_lists_refresh();
    let first_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("first Lists owner");
    let _second_task = terminal.request_x_feed_lists_refresh();
    let second_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("second Lists owner");

    assert_eq!(first_request_id, u64::MAX);
    assert_eq!(second_request_id, 0);

    let ordinary_error = "X list lookup returned HTTP 503".to_string();
    let _accepted = terminal.update_x_feed(Message::XFeedListsLoaded(
        second_request_id,
        XListsMessageResult::new(Err(ordinary_error.clone())),
    ));
    assert_eq!(terminal.x_feed.status, Some((ordinary_error, true)));

    let _third_task = terminal.request_x_feed_lists_refresh();
    let third_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("third Lists owner");
    let _accepted = terminal.update_x_feed(Message::XFeedListsLoaded(
        third_request_id,
        XListsMessageResult::new(Err(
            "X list lookup failed: auth_token=list-handler-secret".to_string()
        )),
    ));
    let (message, is_error) = terminal.x_feed.status.as_ref().expect("Lists error status");
    assert!(*is_error);
    assert!(message.contains("auth_token=<redacted>"), "{message}");
    assert!(!message.contains("list-handler-secret"), "{message}");
}

#[test]
fn lists_result_requires_the_dispatch_user() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");
    terminal.x_feed.lists = vec![test_list("stable", "Stable")];
    let _task = terminal.request_x_feed_lists_refresh();
    let request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("user A Lists owner");
    terminal.x_feed.auth_user = Some(XAuthenticatedUser {
        id: "user-b".to_string(),
        username: "user-b".to_string(),
        name: "User B".to_string(),
    });
    terminal.x_feed.status = Some(("stable status".to_string(), false));

    let _stale = terminal.update_x_feed(Message::XFeedListsLoaded(
        request_id,
        XListsMessageResult::new(Ok(XListsFetchOutcome {
            lists: vec![test_list("stale", "Stale")],
            unavailable_sources: Vec::new(),
        })),
    ));

    assert!(!terminal.x_feed.lists_loading);
    assert_eq!(terminal.x_feed.lists, vec![test_list("stable", "Stable")]);
    assert_eq!(
        terminal.x_feed.status,
        Some(("stable status".to_string(), false))
    );
}

#[test]
fn auth_context_and_manual_lists_results_share_latest_request_ownership() {
    let (mut auth_newer, _) = TradingTerminal::boot();
    configure_x_runtime(&mut auth_newer, "user-a");
    auth_newer.x_feed.lists = vec![test_list("stable", "Stable")];
    let _older_lists_task = auth_newer.request_x_feed_lists_refresh();
    let older_lists_request_id = auth_newer
        .x_feed
        .current_lists_request_id()
        .expect("older manual Lists owner");
    let auth_task = auth_newer.request_x_feed_auth_refresh();
    let auth_request_id = auth_newer
        .x_feed
        .current_auth_request_id()
        .expect("newer auth owner");
    assert_eq!(auth_task.units(), 1);
    assert!(!auth_newer.x_feed.lists_loading);

    let _auth_result = auth_newer.update_x_feed(Message::XFeedAuthLoaded(
        auth_request_id,
        XAuthContextMessageResult::new(Ok((
            XAuthenticatedUser {
                id: "user-a".to_string(),
                username: "user-a".to_string(),
                name: "User A".to_string(),
            },
            XListsFetchOutcome {
                lists: vec![test_list("auth-newer", "Auth Newer")],
                unavailable_sources: Vec::new(),
            },
        ))),
    ));
    assert_eq!(
        auth_newer.x_feed.lists,
        vec![test_list("auth-newer", "Auth Newer")]
    );
    let accepted_status = auth_newer.x_feed.status.clone();

    let _older_result = auth_newer.update_x_feed(Message::XFeedListsLoaded(
        older_lists_request_id,
        XListsMessageResult::new(Ok(XListsFetchOutcome {
            lists: vec![test_list("manual-older", "Manual Older")],
            unavailable_sources: Vec::new(),
        })),
    ));
    assert_eq!(
        auth_newer.x_feed.lists,
        vec![test_list("auth-newer", "Auth Newer")]
    );
    assert_eq!(auth_newer.x_feed.status, accepted_status);

    let (mut manual_newer, _) = TradingTerminal::boot();
    configure_x_runtime(&mut manual_newer, "user-a");
    manual_newer.x_feed.lists = vec![test_list("stable", "Stable")];
    let _older_auth_task = manual_newer.request_x_feed_auth_refresh();
    let older_auth_request_id = manual_newer
        .x_feed
        .current_auth_request_id()
        .expect("older auth owner");
    let _newer_lists_task = manual_newer.request_x_feed_lists_refresh();
    let newer_lists_request_id = manual_newer
        .x_feed
        .current_lists_request_id()
        .expect("newer manual Lists owner");

    let _older_auth_result = manual_newer.update_x_feed(Message::XFeedAuthLoaded(
        older_auth_request_id,
        XAuthContextMessageResult::new(Ok((
            XAuthenticatedUser {
                id: "user-a".to_string(),
                username: "user-a".to_string(),
                name: "User A".to_string(),
            },
            XListsFetchOutcome {
                lists: vec![test_list("auth-older", "Auth Older")],
                unavailable_sources: Vec::new(),
            },
        ))),
    ));
    assert!(manual_newer.x_feed.lists_loading);
    assert_eq!(
        manual_newer.x_feed.lists,
        vec![test_list("stable", "Stable")]
    );

    let _newer_lists_result = manual_newer.update_x_feed(Message::XFeedListsLoaded(
        newer_lists_request_id,
        XListsMessageResult::new(Ok(XListsFetchOutcome {
            lists: vec![test_list("manual-newer", "Manual Newer")],
            unavailable_sources: Vec::new(),
        })),
    ));
    assert!(!manual_newer.x_feed.lists_loading);
    assert_eq!(
        manual_newer.x_feed.lists,
        vec![test_list("manual-newer", "Manual Newer")]
    );
}

#[test]
fn same_source_refresh_fans_out_once_and_settles_once() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");
    terminal.x_feed.instances.clear();
    terminal
        .x_feed
        .instances
        .insert(10, XFeedInstance::new(10, XFeedSource::Following));
    terminal
        .x_feed
        .instances
        .insert(11, XFeedInstance::new(11, XFeedSource::Following));

    let first_task = terminal.request_x_feed_refresh(10, true);
    let second_task = terminal.request_x_feed_refresh(11, true);
    let request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("Following refresh owner");
    assert_eq!(first_task.units(), 1);
    assert_eq!(second_task.units(), 0);

    let page = test_page(
        XFeedSource::Following,
        vec![test_post("post-a", "author-a", None, 1_000)],
        None,
    );
    let followup = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        request_id,
        XFeedPageMessageResult::new(Ok(page)),
    ));

    assert_eq!(followup.units(), 0);
    assert_eq!(terminal.x_feed.instances[&10].posts.len(), 1);
    assert_eq!(terminal.x_feed.instances[&11].posts.len(), 1);
    assert_eq!(
        terminal.x_feed.status,
        Some(("X Feed updated · 1 posts".to_string(), false))
    );
    assert!(
        terminal
            .x_feed
            .current_source_refresh_request_id(&XFeedSource::Following)
            .is_none()
    );

    let accepted_posts = terminal.x_feed.instances[&10].posts.clone();
    let accepted_status = terminal.x_feed.status.clone();
    let _replay = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        request_id,
        XFeedPageMessageResult::new(Err(XFeedRequestError::plain("replayed page failure"))),
    ));
    assert_eq!(terminal.x_feed.instances[&10].posts, accepted_posts);
    assert_eq!(terminal.x_feed.status, accepted_status);
}

#[test]
fn source_result_requires_dispatch_user_and_payload_source() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");
    terminal.x_feed.instances.clear();
    terminal
        .x_feed
        .instances
        .insert(10, XFeedInstance::new(10, XFeedSource::Following));

    let _task = terminal.request_x_feed_refresh(10, true);
    let user_a_request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("user A source owner");
    terminal.x_feed.auth_user = Some(XAuthenticatedUser {
        id: "user-b".to_string(),
        username: "user-b".to_string(),
        name: "User B".to_string(),
    });
    terminal.x_feed.status = Some(("stable status".to_string(), false));

    let _stale_user = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        user_a_request_id,
        XFeedPageMessageResult::new(Ok(test_page(
            XFeedSource::Following,
            vec![test_post("post-a", "author-a", None, 1_000)],
            Some(u64::MAX),
        ))),
    ));
    assert!(terminal.x_feed.instances[&10].posts.is_empty());
    assert_eq!(
        terminal.x_feed.status,
        Some(("stable status".to_string(), false))
    );
    assert!(
        terminal
            .x_feed
            .source_rate_limited_until(&XFeedSource::Following, 0)
            .is_none()
    );
    assert!(
        terminal
            .x_feed
            .current_source_refresh_request_id(&XFeedSource::Following)
            .is_none()
    );

    let _task = terminal.request_x_feed_refresh(10, true);
    let mismatched_page_request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("payload source owner");
    terminal.x_feed.status = Some(("stable status".to_string(), false));
    let mismatched_source = XFeedSource::List {
        id: "list-b".to_string(),
        name: "List B".to_string(),
        private: true,
    };
    let _mismatched_page = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        mismatched_page_request_id,
        XFeedPageMessageResult::new(Ok(test_page(
            mismatched_source,
            vec![test_post("post-b", "author-b", None, 2_000)],
            Some(u64::MAX),
        ))),
    ));
    assert!(terminal.x_feed.instances[&10].posts.is_empty());
    assert_eq!(
        terminal.x_feed.status,
        Some(("stable status".to_string(), false))
    );
    assert!(
        terminal
            .x_feed
            .source_rate_limited_until(&XFeedSource::Following, 0)
            .is_none()
    );
}

#[test]
fn accepted_source_errors_preserve_ordinary_text_and_redact_secrets() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");
    terminal.x_feed.instances.clear();
    terminal
        .x_feed
        .instances
        .insert(10, XFeedInstance::new(10, XFeedSource::Following));

    let _task = terminal.request_x_feed_refresh(10, true);
    let ordinary_request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("ordinary error owner");
    let ordinary_error = "X feed request returned HTTP 503".to_string();
    let _accepted = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        ordinary_request_id,
        XFeedPageMessageResult::new(Err(XFeedRequestError::plain(ordinary_error.clone()))),
    ));
    assert_eq!(
        terminal.x_feed.instances[&10].last_error,
        Some(ordinary_error.clone())
    );
    assert_eq!(terminal.x_feed.status, Some((ordinary_error, true)));

    let _task = terminal.request_x_feed_refresh(10, true);
    let secret_request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("secret error owner");
    let _accepted = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        secret_request_id,
        XFeedPageMessageResult::new(Err(XFeedRequestError::plain(
            "X feed request failed: access_token=page-handler-secret",
        ))),
    ));
    let (message, is_error) = terminal.x_feed.status.as_ref().expect("page error status");
    assert!(*is_error);
    assert!(message.contains("access_token=<redacted>"), "{message}");
    assert!(!message.contains("page-handler-secret"), "{message}");
    assert_eq!(
        terminal.x_feed.instances[&10].last_error.as_deref(),
        Some(message.as_str())
    );
}

#[test]
fn profile_image_result_requires_exact_profile_and_url_owner() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal
        .x_feed
        .set_noncredential_request_allocators_for_test(0, 0, u64::MAX - 1);
    let image_url_a = "https://images.invalid/author-a.png";
    let image_url_b = "https://images.invalid/author-b.png";

    let first_task = terminal.schedule_x_profile_image_fetches(&test_page(
        XFeedSource::Following,
        vec![test_post("post-a", "author-a", Some(image_url_a), 1_000)],
        None,
    ));
    let profile_key = "id:author-a";
    let first_request_id = terminal.x_feed.author_profiles[profile_key].image_request_id;
    assert_eq!(first_task.units(), 1);
    assert_eq!(first_request_id, u64::MAX);

    let second_task = terminal.schedule_x_profile_image_fetches(&test_page(
        XFeedSource::Following,
        vec![test_post("post-b", "author-a", Some(image_url_b), 2_000)],
        None,
    ));
    let second_request_id = terminal.x_feed.author_profiles[profile_key].image_request_id;
    assert_eq!(second_task.units(), 1);
    assert_ne!(second_request_id, first_request_id);
    assert_ne!(second_request_id, 0);

    let image_bytes_a = b"\x89PNG\r\n\x1A\nimage-a".to_vec();
    let _stale = terminal.update_x_feed(Message::XProfileImageLoaded(
        first_request_id,
        XProfileImageMessageResult::new(Ok(image_bytes_a)),
    ));
    let profile = &terminal.x_feed.author_profiles[profile_key];
    assert!(profile.image_handle.is_none());
    assert_eq!(profile.image_request_id, second_request_id);
    assert_eq!(profile.image_loading_url.as_deref(), Some(image_url_b));

    let image_bytes_b = b"\x89PNG\r\n\x1A\nimage-b".to_vec();
    let _accepted = terminal.update_x_feed(Message::XProfileImageLoaded(
        second_request_id,
        XProfileImageMessageResult::new(Ok(image_bytes_b)),
    ));
    let profile = &terminal.x_feed.author_profiles[profile_key];
    assert!(profile.image_handle.is_some());
    assert_eq!(profile.image_request_id, 0);
    assert!(profile.image_loading_url.is_none());

    let _replay = terminal.update_x_feed(Message::XProfileImageLoaded(
        second_request_id,
        XProfileImageMessageResult::new(Err("replayed image error".to_string())),
    ));
    assert!(
        terminal.x_feed.author_profiles[profile_key]
            .image_handle
            .is_some()
    );
}

#[test]
fn config_clear_preserves_all_x_request_allocators_without_owners() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_x_runtime(&mut terminal, "user-a");
    terminal.x_feed.instances.clear();
    terminal
        .x_feed
        .instances
        .insert(10, XFeedInstance::new(10, XFeedSource::Following));

    let _old_lists_task = terminal.request_x_feed_lists_refresh();
    let old_lists_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("old Lists owner");
    let _old_source_task = terminal.request_x_feed_refresh(10, true);
    let old_source_request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("old source owner");
    let _old_image_task = terminal.schedule_x_profile_image_fetches(&test_page(
        XFeedSource::Following,
        vec![test_post(
            "post-a",
            "author-a",
            Some("https://images.invalid/old.png"),
            1_000,
        )],
        None,
    ));
    let old_image_request_id = terminal.x_feed.author_profiles["id:author-a"].image_request_id;

    let _clear_task = terminal.apply_config_clear_to_runtime(config::ClearConfigSummary {
        files_removed: 0,
        file_cleanup_failed: false,
        keychain_entries_cleared: 0,
        warnings: Vec::new(),
    });
    assert!(!terminal.x_feed.lists_loading);
    assert!(!terminal.x_feed.loading());

    configure_x_runtime(&mut terminal, "user-b");
    terminal
        .x_feed
        .instances
        .insert(20, XFeedInstance::new(20, XFeedSource::Following));
    let _new_lists_task = terminal.request_x_feed_lists_refresh();
    let new_lists_request_id = terminal
        .x_feed
        .current_lists_request_id()
        .expect("new Lists owner");
    let _new_source_task = terminal.request_x_feed_refresh(20, true);
    let new_source_request_id = terminal
        .x_feed
        .current_source_refresh_request_id(&XFeedSource::Following)
        .expect("new source owner");
    let _new_image_task = terminal.schedule_x_profile_image_fetches(&test_page(
        XFeedSource::Following,
        vec![test_post(
            "post-b",
            "author-b",
            Some("https://images.invalid/new.png"),
            2_000,
        )],
        None,
    ));
    let new_image_request_id = terminal.x_feed.author_profiles["id:author-b"].image_request_id;

    assert_ne!(new_lists_request_id, old_lists_request_id);
    assert_ne!(new_source_request_id, old_source_request_id);
    assert_ne!(new_image_request_id, old_image_request_id);

    let stable_status = terminal.x_feed.status.clone();
    let _old_lists = terminal.update_x_feed(Message::XFeedListsLoaded(
        old_lists_request_id,
        XListsMessageResult::new(Err("old Lists result".to_string())),
    ));
    let _old_source = terminal.update_x_feed(Message::XFeedLoaded(
        XFeedSource::Following,
        old_source_request_id,
        XFeedPageMessageResult::new(Err(XFeedRequestError::plain("old source result"))),
    ));
    let _old_image = terminal.update_x_feed(Message::XProfileImageLoaded(
        old_image_request_id,
        XProfileImageMessageResult::new(Ok(b"\x89PNG\r\n\x1A\nold".to_vec())),
    ));

    assert!(terminal.x_feed.lists_loading);
    assert!(
        terminal
            .x_feed
            .source_refresh_in_flight(&XFeedSource::Following)
    );
    assert_eq!(terminal.x_feed.status, stable_status);
    assert!(
        terminal.x_feed.author_profiles["id:author-b"]
            .image_handle
            .is_none()
    );
}
