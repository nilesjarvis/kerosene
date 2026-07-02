use super::openrouter_key_status_message;
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config;
use crate::message::Message;
use crate::openrouter_api::OpenRouterKeyStatus;

fn configure_encrypted_openrouter_key(
    terminal: &mut TradingTerminal,
    openrouter_key: &str,
    unlocked: bool,
) {
    terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
    terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secret_password = sensitive_string("test-password");
    let mut payload = config::SecretPayload::from_credentials(&[], "hydro-key", "");
    payload.set_global_openrouter_api_key(openrouter_key);
    terminal.encrypted_secrets = Some(
        config::encrypt_secrets(&payload, &terminal.encrypted_secret_password)
            .expect("test encrypted payload"),
    );
    terminal.encrypted_secrets_unlocked = unlocked;
    terminal.hydromancer_api_key = sensitive_string("hydro-key");
    terminal.secret_migration_save_blocked = false;
    terminal.secret_store_status = None;
}

#[test]
fn openrouter_save_commits_after_encrypted_persistence_succeeds() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_encrypted_openrouter_key(&mut terminal, "old-openrouter", true);
    terminal.openrouter_api_key = sensitive_string("old-openrouter");
    terminal.openrouter_key_input = sensitive_string("  new-openrouter  ");
    terminal.openrouter_key_generation = 4;
    terminal.openrouter_key_status = Some(("stale status".to_string(), true));
    terminal.config_save_due_at = None;

    let _task = terminal.update_openrouter(Message::SaveOpenRouterKey);

    assert_eq!(terminal.openrouter_api_key.as_str(), "new-openrouter");
    assert_eq!(terminal.openrouter_key_generation, 5);
    let payload = config::decrypt_secrets(
        terminal
            .encrypted_secrets
            .as_ref()
            .expect("encrypted secrets should be rewritten"),
        &terminal.encrypted_secret_password,
    )
    .expect("encrypted secrets should decrypt");
    assert_eq!(payload.global_hydromancer_api_key(), "hydro-key");
    assert_eq!(payload.global_openrouter_api_key(), "new-openrouter");
    assert!(!terminal.secret_migration_save_blocked);
    assert!(terminal.config_save_due_at.is_some());
    let (message, is_error) = terminal
        .openrouter_key_status
        .as_ref()
        .expect("key check should be pending");
    assert!(!*is_error);
    assert!(message.contains("Checking key"));
}

#[test]
fn openrouter_save_failure_preserves_live_key_and_generation() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_encrypted_openrouter_key(&mut terminal, "old-openrouter", false);
    terminal.openrouter_api_key = sensitive_string("old-openrouter");
    terminal.openrouter_key_input = sensitive_string("new-openrouter");
    terminal.openrouter_key_generation = 4;
    terminal.config_save_due_at = None;

    let _task = terminal.update_openrouter(Message::SaveOpenRouterKey);

    assert_eq!(terminal.openrouter_api_key.as_str(), "old-openrouter");
    assert_eq!(terminal.openrouter_key_input.as_str(), "new-openrouter");
    assert_eq!(terminal.openrouter_key_generation, 4);
    assert!(terminal.secret_migration_save_blocked);
    assert!(terminal.config_save_due_at.is_none());
    let (message, is_error) = terminal.secret_store_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(message.contains("Unlock encrypted credentials"));
}

#[test]
fn openrouter_key_clear_resets_status_without_key_check() {
    let (mut terminal, _) = TradingTerminal::boot();
    configure_encrypted_openrouter_key(&mut terminal, "old-openrouter", true);
    terminal.openrouter_api_key = sensitive_string("old-openrouter");
    terminal.openrouter_key_input = sensitive_string("");
    terminal.openrouter_key_generation = 4;
    terminal.openrouter_key_status = Some(("Key valid".to_string(), false));

    let _task = terminal.update_openrouter(Message::SaveOpenRouterKey);

    assert_eq!(terminal.openrouter_api_key.as_str(), "");
    assert_eq!(terminal.openrouter_key_generation, 5);
    assert!(terminal.openrouter_key_status.is_none());
    let payload = config::decrypt_secrets(
        terminal
            .encrypted_secrets
            .as_ref()
            .expect("encrypted secrets should be rewritten"),
        &terminal.encrypted_secret_password,
    )
    .expect("encrypted secrets should decrypt");
    assert_eq!(payload.global_openrouter_api_key(), "");
}

#[test]
fn openrouter_key_check_result_is_ignored_for_stale_generation() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_api_key = sensitive_string("live-key");
    terminal.openrouter_key_generation = 5;
    terminal.openrouter_key_status = Some(("Checking key...".to_string(), false));

    let _task = terminal.update_openrouter(Message::OpenRouterKeyChecked(
        4,
        Err("stale error".to_string()),
    ));

    assert_eq!(
        terminal.openrouter_key_status,
        Some(("Checking key...".to_string(), false))
    );
}

#[test]
fn openrouter_key_check_result_updates_status_for_current_generation() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_api_key = sensitive_string("live-key");
    terminal.openrouter_key_generation = 5;
    terminal.openrouter_key_status = Some(("Checking key...".to_string(), false));

    let _task = terminal.update_openrouter(Message::OpenRouterKeyChecked(
        5,
        Ok(OpenRouterKeyStatus {
            usage_usd: 25.5,
            limit_usd: Some(100.0),
            limit_remaining_usd: Some(74.5),
            is_free_tier: false,
        }),
    ));

    let (message, is_error) = terminal.openrouter_key_status.as_ref().expect("status");
    assert!(!*is_error);
    assert_eq!(
        message,
        "Key valid — $25.50 used, $74.50 of $100.00 limit left"
    );

    let _task = terminal.update_openrouter(Message::OpenRouterKeyChecked(
        5,
        Err("OpenRouter key check HTTP 401 (invalid or disabled API key): no auth".to_string()),
    ));

    let (message, is_error) = terminal.openrouter_key_status.as_ref().expect("status");
    assert!(*is_error);
    assert!(message.contains("HTTP 401"));
}

#[test]
fn openrouter_model_change_updates_state_and_schedules_config_save() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.config_save_due_at = None;

    let _task = terminal.update_openrouter(Message::OpenRouterModelChanged(
        "anthropic/claude-sonnet-4.5".to_string(),
    ));

    assert_eq!(terminal.openrouter_model, "anthropic/claude-sonnet-4.5");
    assert_eq!(
        terminal.openrouter_model_for_task(),
        "anthropic/claude-sonnet-4.5"
    );
    assert!(terminal.config_save_due_at.is_some());
}

#[test]
fn openrouter_model_for_task_falls_back_to_auto_router() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.openrouter_model = "  ".to_string();

    assert_eq!(
        terminal.openrouter_model_for_task(),
        crate::openrouter_api::DEFAULT_OPENROUTER_MODEL
    );
}

#[test]
fn openrouter_key_status_message_formats_limits_and_free_tier() {
    let unlimited = openrouter_key_status_message(&OpenRouterKeyStatus {
        usage_usd: 0.0,
        limit_usd: None,
        limit_remaining_usd: None,
        is_free_tier: true,
    });
    assert_eq!(unlimited, "Key valid — $0.00 used (free tier)");

    let remaining_only = openrouter_key_status_message(&OpenRouterKeyStatus {
        usage_usd: 1.25,
        limit_usd: None,
        limit_remaining_usd: Some(3.75),
        is_free_tier: false,
    });
    assert_eq!(remaining_only, "Key valid — $1.25 used, $3.75 left");
}
