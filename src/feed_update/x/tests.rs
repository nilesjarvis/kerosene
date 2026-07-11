use super::*;
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config;
use crate::x_feed::{XAuthenticatedUser, XListsFetchOutcome, XOAuthTokenRefresh};

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
