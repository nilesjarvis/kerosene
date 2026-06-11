use crate::config::secrets::{
    clear_legacy_keychain_entries_for_payload, load_keychain_secret_payload, load_profile_secrets,
    push_secret_warning, store_secret_payload,
};
use crate::config::{
    AccountProfile, CredentialStorageMode, KeroseneConfig, SecretPayload, new_secret_id,
};
use zeroize::Zeroize;

mod payload;

use self::payload::{apply_secret_payload, merge_missing_plaintext_secrets_into_payload};

// ---------------------------------------------------------------------------
// Secret Storage Hydration
// ---------------------------------------------------------------------------

pub(super) fn load_configured_secrets(config: &mut KeroseneConfig) {
    match config.credential_storage_mode {
        CredentialStorageMode::OsKeychain => load_os_keychain_secrets(config),
        CredentialStorageMode::EncryptedConfig => lock_encrypted_config_secrets(config),
    }
}

fn load_os_keychain_secrets(config: &mut KeroseneConfig) {
    load_os_keychain_secrets_with(
        config,
        load_keychain_secret_payload,
        store_secret_payload,
        clear_legacy_keychain_entries_for_payload,
        load_profile_secrets,
        push_secret_warning,
    );
}

fn load_os_keychain_secrets_with(
    config: &mut KeroseneConfig,
    mut load_payload: impl FnMut() -> Result<Option<SecretPayload>, String>,
    mut store_payload: impl FnMut(&SecretPayload) -> Result<(), String>,
    mut clear_legacy_entries: impl FnMut(&SecretPayload) -> Result<(), String>,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
    match load_payload() {
        Ok(Some(mut payload)) => {
            let mut bundle_update_succeeded = true;
            if merge_missing_plaintext_secrets_into_payload(config, &mut payload)
                && let Err(error) = store_payload(&payload)
            {
                bundle_update_succeeded = false;
                push_warning(format!("Credential bundle migration failed: {error}"));
            }
            apply_secret_payload(config, &payload);
            if bundle_update_succeeded && let Err(error) = clear_legacy_entries(&payload) {
                push_warning(format!(
                    "Legacy OS keychain cleanup failed after bundle load: {error}"
                ));
            }
            return;
        }
        Ok(None) => {}
        Err(error) => {
            normalize_legacy_plaintext_secrets(config);
            push_warning(format!(
                "Credential bundle read failed: {error}; OS keychain credentials were left unchanged. Re-enter credentials or switch to encrypted config in Settings > Storage if the problem persists"
            ));
            return;
        }
    }

    load_legacy_os_keychain_secrets_with_warnings(config, &mut load_profile, &mut push_warning);

    let payload = SecretPayload::from_credentials(
        &config.accounts,
        &config.hydromancer_api_key,
        &config.hyperdash_api_key,
        &config.x_bearer_token,
    );
    if !payload.is_empty() {
        match store_payload(&payload) {
            Ok(()) => {
                if let Err(error) = clear_legacy_entries(&payload) {
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
            Err(error) => push_warning(format!("Credential bundle migration failed: {error}")),
        }
    }
}

fn load_legacy_os_keychain_secrets_with_warnings(
    config: &mut KeroseneConfig,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
    normalize_legacy_plaintext_secrets(config);

    let Some(active_index) = active_legacy_profile_index(config) else {
        return;
    };
    if config.accounts[active_index].agent_key.trim().is_empty() {
        let profile = &mut config.accounts[active_index];
        if let Err(error) = load_profile(profile) {
            push_warning(format!("{}: {error}", profile.name));
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
}

fn normalize_legacy_plaintext_secrets(config: &mut KeroseneConfig) {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }
    }

    let mut legacy_hydromancer_key = std::mem::take(&mut config.hydromancer_api_key);
    for profile in &mut config.accounts {
        if !profile.hydromancer_api_key.trim().is_empty() {
            if legacy_hydromancer_key.trim().is_empty() {
                legacy_hydromancer_key = profile.hydromancer_api_key.clone();
            }
            profile.hydromancer_api_key.zeroize();
        }
    }
    config.hydromancer_api_key = legacy_hydromancer_key.to_string().into();

    let legacy_hyperdash_key = std::mem::take(&mut config.hyperdash_api_key);
    config.hyperdash_api_key = legacy_hyperdash_key.to_string().into();
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

fn lock_encrypted_config_secrets(config: &mut KeroseneConfig) {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }
        profile.agent_key.zeroize();
        profile.hydromancer_api_key.zeroize();
    }
    config.hydromancer_api_key.zeroize();
    config.hyperdash_api_key.zeroize();
    config.x_bearer_token.zeroize();

    if config.encrypted_secrets.is_some() {
        push_secret_warning(
            "Encrypted credentials are locked; unlock them in Settings > Storage".to_string(),
        );
    } else {
        push_secret_warning(
            "Encrypted credential storage is selected but no encrypted credentials are saved"
                .to_string(),
        );
    }
}

#[cfg(test)]
mod tests;
