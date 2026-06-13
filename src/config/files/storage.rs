use crate::config::secrets::{
    clear_all_keychain_secrets, clear_legacy_keychain_entries_for_payload,
    clear_profile_secrets_by_id, load_global_secrets, load_keychain_secret_payload,
    load_profile_secrets, push_secret_warning, store_secret_payload,
    validate_encrypted_secrets_metadata,
};
use crate::config::{
    AccountProfile, CredentialStorageMode, KeroseneConfig, SecretPayload, new_secret_id,
};
use zeroize::Zeroize;

mod payload;

use self::payload::{
    applied_secret_payload_for_legacy_cleanup, apply_secret_payload,
    apply_secret_payload_preserving_missing_plaintext, bind_legacy_unbound_profile_keys_to_wallets,
    merge_missing_plaintext_secrets_into_payload,
};

// ---------------------------------------------------------------------------
// Secret Storage Hydration
// ---------------------------------------------------------------------------

struct KeychainCleanupHooks<ClearLegacyEntries, ClearPendingProfile> {
    clear_legacy_entries: ClearLegacyEntries,
    clear_pending_profile: ClearPendingProfile,
}

pub(super) fn load_configured_secrets(config: &mut KeroseneConfig) {
    match config.credential_storage_mode {
        CredentialStorageMode::OsKeychain => {
            if config.pending_keychain_cleanup_all {
                config.pending_keychain_cleanup_all = false;
                config.secret_cleanup_state_dirty = true;
            }
            load_os_keychain_secrets(config);
        }
        CredentialStorageMode::EncryptedConfig => {
            load_encrypted_config_secrets_with(
                config,
                clear_all_keychain_secrets,
                clear_profile_secrets_by_id,
                push_secret_warning,
            );
        }
    }
}

fn load_encrypted_config_secrets_with(
    config: &mut KeroseneConfig,
    _clear_all_keychain: impl FnOnce(&[AccountProfile]) -> Result<(), String>,
    _clear_pending_profile: impl FnMut(&str) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
    if config.pending_keychain_cleanup_all || !config.pending_keychain_profile_deletions.is_empty()
    {
        push_warning(
            "Pending OS keychain cleanup was deferred until encrypted credentials are unlocked"
                .to_string(),
        );
    }
    lock_encrypted_config_secrets_with(config, push_warning);
}

#[cfg(test)]
fn retry_pending_keychain_cleanup_all(
    config: &mut KeroseneConfig,
    clear_all_keychain: impl FnOnce(&[AccountProfile]) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
    if !config.pending_keychain_cleanup_all {
        return;
    }

    let profiles = keychain_cleanup_profiles_for_config(config);
    match clear_all_keychain(&profiles) {
        Ok(()) => {
            config.pending_keychain_cleanup_all = false;
            config.pending_keychain_profile_deletions.clear();
            config.secret_cleanup_state_dirty = true;
        }
        Err(error) => {
            let redacted = redact_keychain_cleanup_profile_ids(error, &profiles);
            push_warning(format!(
                "Pending OS keychain cleanup failed and will retry later: {redacted}"
            ));
        }
    }
}

#[cfg(test)]
fn keychain_cleanup_profiles_for_config(config: &KeroseneConfig) -> Vec<AccountProfile> {
    let mut profiles = config.accounts.clone();
    for secret_id in &config.pending_keychain_profile_deletions {
        let secret_id = secret_id.trim();
        if secret_id.is_empty()
            || profiles
                .iter()
                .any(|profile| profile.secret_id.as_str() == secret_id)
        {
            continue;
        }
        profiles.push(AccountProfile {
            secret_id: secret_id.to_string(),
            name: String::new(),
            wallet_address: String::new(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        });
    }
    profiles
}

#[cfg(test)]
fn redact_keychain_cleanup_profile_ids(error: String, profiles: &[AccountProfile]) -> String {
    profiles.iter().fold(error, |message, profile| {
        let secret_id = profile.secret_id.trim();
        if secret_id.is_empty() {
            message
        } else {
            message.replace(secret_id, "<redacted-profile>")
        }
    })
}

fn load_os_keychain_secrets(config: &mut KeroseneConfig) {
    load_os_keychain_secrets_with(
        config,
        load_keychain_secret_payload,
        store_secret_payload,
        KeychainCleanupHooks {
            clear_legacy_entries: clear_legacy_keychain_entries_for_payload,
            clear_pending_profile: clear_profile_secrets_by_id,
        },
        load_profile_secrets,
        load_global_secrets,
        push_secret_warning,
    );
}

fn load_os_keychain_secrets_with(
    config: &mut KeroseneConfig,
    mut load_payload: impl FnMut() -> Result<Option<SecretPayload>, String>,
    mut store_payload: impl FnMut(&SecretPayload) -> Result<(), String>,
    mut cleanup_hooks: KeychainCleanupHooks<
        impl FnMut(&SecretPayload) -> Result<(), String>,
        impl FnMut(&str) -> Result<(), String>,
    >,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
    mut load_global: impl FnMut(
        &mut zeroize::Zeroizing<String>,
        &mut zeroize::Zeroizing<String>,
        &mut zeroize::Zeroizing<String>,
    ) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
    retry_pending_keychain_profile_deletions(
        config,
        &mut cleanup_hooks.clear_pending_profile,
        &mut push_warning,
    );

    match load_payload() {
        Ok(Some(payload)) => {
            if let Err(error) = normalize_legacy_plaintext_secrets(config) {
                config.secret_migration_save_blocked = true;
                push_warning(format!(
                    "Legacy plaintext credential migration failed: {error}; config saves are paused until credentials are saved to a working store"
                ));
                return;
            }
            let mut merged_payload = payload.clone();
            let plaintext_merge_changed =
                merge_missing_plaintext_secrets_into_payload(config, &mut merged_payload);
            let legacy_binding_changed =
                bind_legacy_unbound_profile_keys_to_wallets(config, &mut merged_payload);
            let active_legacy_profile_changed = match merge_active_legacy_profile_key_into_payload(
                config,
                &mut merged_payload,
                &mut load_profile,
            ) {
                Ok(changed) => changed,
                Err(error) => {
                    config.secret_migration_save_blocked = true;
                    push_warning(format!(
                        "Active legacy account credential read failed: {error}; config saves are paused until credentials are saved to a working store"
                    ));
                    apply_secret_payload_preserving_missing_plaintext(config, &payload);
                    return;
                }
            };
            let should_store =
                plaintext_merge_changed || legacy_binding_changed || active_legacy_profile_changed;
            if should_store && let Err(error) = store_payload(&merged_payload) {
                config.secret_migration_save_blocked = true;
                push_warning(format!(
                    "Credential bundle migration failed: {error}; config saves are paused until credentials are saved to a working store"
                ));
                apply_secret_payload_preserving_missing_plaintext(config, &payload);
                return;
            }
            let cleanup_payload =
                applied_secret_payload_for_legacy_cleanup(config, &merged_payload);
            apply_secret_payload(config, &merged_payload);
            if let Err(error) = (cleanup_hooks.clear_legacy_entries)(&cleanup_payload) {
                push_warning(format!(
                    "Legacy OS keychain cleanup failed after bundle load: {error}"
                ));
            }
            return;
        }
        Ok(None) => {}
        Err(error) => {
            let has_plaintext_secrets = has_legacy_plaintext_secrets(config);
            let normalization_error = normalize_legacy_plaintext_secrets(config).err();
            if has_plaintext_secrets {
                config.secret_migration_save_blocked = true;
                let mut warning = format!(
                    "Credential bundle read failed: {error}; OS keychain credentials were left unchanged and config saves are paused until credentials are saved to a working store"
                );
                if let Some(normalization_error) = normalization_error {
                    warning.push_str("; legacy plaintext migration also failed: ");
                    warning.push_str(&normalization_error);
                }
                push_warning(warning);
            } else {
                push_warning(format!(
                    "Credential bundle read failed: {error}; OS keychain credentials were left unchanged. Re-enter credentials or switch to encrypted config in Settings > Storage if the problem persists"
                ));
            }
            return;
        }
    }

    if !load_legacy_os_keychain_secrets_with_warnings(
        config,
        &mut load_profile,
        &mut load_global,
        &mut push_warning,
    ) {
        config.secret_migration_save_blocked = true;
        return;
    }

    let payload = SecretPayload::from_credentials(
        &config.accounts,
        &config.hydromancer_api_key,
        &config.hyperdash_api_key,
        &config.x_bearer_token,
    );
    if !payload.is_empty() {
        match store_payload(&payload) {
            Ok(()) => {
                if let Err(error) = (cleanup_hooks.clear_legacy_entries)(&payload) {
                    push_warning(format!(
                        "Credential bundle migrated, but legacy OS keychain cleanup failed: {error}"
                    ));
                } else {
                    push_warning(
                        "Legacy OS keychain credentials were migrated to the current storage bundle"
                            .to_string(),
                    );
                }
            }
            Err(error) => {
                config.secret_migration_save_blocked = true;
                push_warning(format!(
                    "Credential bundle migration failed: {error}; config saves are paused until credentials are saved to a working store"
                ));
            }
        }
    }
}

fn retry_pending_keychain_profile_deletions(
    config: &mut KeroseneConfig,
    mut clear_pending_profile: impl FnMut(&str) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
    if config.pending_keychain_profile_deletions.is_empty() {
        return;
    }

    let pending = std::mem::take(&mut config.pending_keychain_profile_deletions);
    let mut cleaned_any = false;
    for secret_id in pending {
        match clear_pending_profile(&secret_id) {
            Ok(()) => {
                cleaned_any = true;
            }
            Err(error) => {
                let redacted = if secret_id.trim().is_empty() {
                    error
                } else {
                    error.replace(&secret_id, "<redacted-profile>")
                };
                config.pending_keychain_profile_deletions.push(secret_id);
                push_warning(format!(
                    "Pending OS keychain account deletion cleanup failed and will retry later: {redacted}"
                ));
            }
        }
    }
    if cleaned_any {
        config.secret_cleanup_state_dirty = true;
    }
}

fn load_legacy_os_keychain_secrets_with_warnings(
    config: &mut KeroseneConfig,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
    mut load_global: impl FnMut(
        &mut zeroize::Zeroizing<String>,
        &mut zeroize::Zeroizing<String>,
        &mut zeroize::Zeroizing<String>,
    ) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) -> bool {
    if let Err(error) = normalize_legacy_plaintext_secrets(config) {
        push_warning(format!(
            "Legacy plaintext credential migration failed: {error}"
        ));
        return false;
    }
    if let Err(error) = load_global(
        &mut config.hydromancer_api_key,
        &mut config.hyperdash_api_key,
        &mut config.x_bearer_token,
    ) {
        push_warning(format!("Legacy shared credential read failed: {error}"));
        return false;
    }

    let Some(active_index) = active_legacy_profile_index(config) else {
        return true;
    };
    if config.accounts[active_index].agent_key.trim().is_empty()
        || config.hydromancer_api_key.trim().is_empty()
    {
        let profile = &mut config.accounts[active_index];
        if let Err(error) = load_profile(profile) {
            push_warning(format!(
                "Active legacy account credential read failed: {error}"
            ));
            return false;
        }
        if let Err(error) = normalize_legacy_plaintext_secrets(config) {
            push_warning(format!(
                "Legacy plaintext credential migration failed: {error}"
            ));
            return false;
        }
    }

    if !config.accounts[active_index].agent_key.trim().is_empty()
        && has_deferred_legacy_keychain_secrets(config, active_index)
    {
        push_warning(
            concat!(
                "Only the active legacy account key was read on startup to avoid repeated ",
                "macOS Keychain prompts; other legacy account keys will migrate when you ",
                "switch to them."
            )
            .to_string(),
        );
    }
    true
}

fn merge_active_legacy_profile_key_into_payload(
    config: &mut KeroseneConfig,
    payload: &mut SecretPayload,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
) -> Result<bool, String> {
    let Some(active_index) = active_legacy_profile_index(config) else {
        return Ok(false);
    };
    let Some(profile) = config.accounts.get(active_index) else {
        return Ok(false);
    };

    let secret_id = profile.secret_id.trim().to_string();
    if secret_id.is_empty()
        || payload
            .profile_agent_key_for_wallet(&secret_id, &profile.wallet_address)
            .is_some()
        || payload.profile_agent_key_binding_mismatches(&secret_id, &profile.wallet_address)
    {
        return Ok(false);
    }

    let wallet_address = profile.wallet_address.clone();
    let mut legacy_profile = profile.clone();
    load_profile(&mut legacy_profile)?;
    if legacy_profile.agent_key.trim().is_empty() {
        return Ok(false);
    }

    let agent_key = legacy_profile.agent_key.clone();
    let changed =
        payload.upsert_profile_agent_key_for_wallet(&secret_id, Some(&wallet_address), &agent_key);
    if changed && let Some(profile) = config.accounts.get_mut(active_index) {
        profile.agent_key.zeroize();
        profile.agent_key = agent_key;
    }
    Ok(changed)
}

fn normalize_legacy_plaintext_secrets(config: &mut KeroseneConfig) -> Result<(), String> {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }
    }

    let mut legacy_hydromancer_key = config.hydromancer_api_key.trim().to_string();
    for profile in &config.accounts {
        let profile_hydromancer_key = profile.hydromancer_api_key.trim();
        if profile_hydromancer_key.is_empty() {
            continue;
        }
        if legacy_hydromancer_key.is_empty() {
            legacy_hydromancer_key = profile_hydromancer_key.to_string();
        } else if legacy_hydromancer_key != profile_hydromancer_key {
            return Err(
                "multiple legacy Hydromancer API keys were found; choose and save the intended global key before migration"
                    .to_string(),
            );
        }
    }
    for profile in &mut config.accounts {
        profile.hydromancer_api_key.zeroize();
    }
    config.hydromancer_api_key.zeroize();
    config.hydromancer_api_key = legacy_hydromancer_key.into();

    let legacy_hyperdash_key = std::mem::take(&mut config.hyperdash_api_key);
    config.hyperdash_api_key = legacy_hyperdash_key.to_string().into();
    Ok(())
}

fn has_legacy_plaintext_secrets(config: &KeroseneConfig) -> bool {
    !config.agent_key.trim().is_empty()
        || !config.hydromancer_api_key.trim().is_empty()
        || !config.hyperdash_api_key.trim().is_empty()
        || !config.x_bearer_token.trim().is_empty()
        || config.accounts.iter().any(|profile| {
            !profile.agent_key.trim().is_empty() || !profile.hydromancer_api_key.trim().is_empty()
        })
}

fn active_legacy_profile_index(config: &KeroseneConfig) -> Option<usize> {
    if config.accounts.is_empty() {
        None
    } else {
        Some(config.active_account_index.min(config.accounts.len() - 1))
    }
}

fn has_deferred_legacy_keychain_secrets(config: &KeroseneConfig, active_index: usize) -> bool {
    config
        .accounts
        .iter()
        .enumerate()
        .any(|(index, profile)| index != active_index && profile.agent_key.trim().is_empty())
}

fn clear_plaintext_secret_fields(config: &mut KeroseneConfig) {
    for profile in &mut config.accounts {
        profile.agent_key.zeroize();
        profile.hydromancer_api_key.zeroize();
    }
    config.hydromancer_api_key.zeroize();
    config.hyperdash_api_key.zeroize();
    config.x_bearer_token.zeroize();
}

fn lock_encrypted_config_secrets_with(
    config: &mut KeroseneConfig,
    mut push_warning: impl FnMut(String),
) {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }
    }

    if config.encrypted_secrets.is_none() && has_legacy_plaintext_secrets(config) {
        config.secret_migration_save_blocked = true;
        clear_plaintext_secret_fields(config);
        push_warning(
            concat!(
                "Encrypted credential storage is selected but no encrypted credentials are saved; ",
                "plaintext credentials were not loaded into the running session and config saves are paused until ",
                "credentials are saved to a working store"
            )
            .to_string(),
        );
        return;
    }

    if let Some(encrypted) = config.encrypted_secrets.as_ref()
        && let Err(error) = validate_encrypted_secrets_metadata(encrypted)
    {
        if has_legacy_plaintext_secrets(config) {
            config.secret_migration_save_blocked = true;
            clear_plaintext_secret_fields(config);
            push_warning(format!(
                "Encrypted credential storage metadata is invalid: {error}; plaintext credentials were not loaded into the running session and config saves are paused until credentials are saved to a working store"
            ));
        } else {
            push_warning(format!(
                "Encrypted credential storage metadata is invalid: {error}; re-enter credentials or switch storage mode in Settings > Storage"
            ));
        }
        return;
    }

    clear_plaintext_secret_fields(config);

    if config.encrypted_secrets.is_some() {
        push_warning(
            "Encrypted credentials are locked; unlock them in Settings > Storage".to_string(),
        );
    } else {
        push_warning(
            "Encrypted credential storage is selected but no encrypted credentials are saved"
                .to_string(),
        );
    }
}

#[cfg(test)]
mod tests;
