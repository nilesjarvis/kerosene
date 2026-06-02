use crate::config::secrets::{
    load_keychain_secret_payload, load_profile_secrets, push_secret_warning, store_secret_payload,
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
    match load_keychain_secret_payload() {
        Ok(Some(mut payload)) => {
            if merge_missing_plaintext_secrets_into_payload(config, &mut payload)
                && let Err(error) = store_secret_payload(&payload)
            {
                push_secret_warning(format!("Credential bundle migration failed: {error}"));
            }
            apply_secret_payload(config, &payload);
            return;
        }
        Ok(None) => {}
        Err(error) => {
            push_secret_warning(format!("Credential bundle read failed: {error}"));
            return;
        }
    }

    load_legacy_os_keychain_secrets(config);

    let payload = SecretPayload::from_credentials(
        &config.accounts,
        &config.hydromancer_api_key,
        &config.hyperdash_api_key,
        &config.x_bearer_token,
    );
    if !payload.is_empty()
        && let Err(error) = store_secret_payload(&payload)
    {
        push_secret_warning(format!("Credential bundle migration failed: {error}"));
    }
}

fn load_legacy_os_keychain_secrets(config: &mut KeroseneConfig) {
    load_legacy_os_keychain_secrets_with(config, load_profile_secrets);
}

fn load_legacy_os_keychain_secrets_with(
    config: &mut KeroseneConfig,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
) {
    load_legacy_os_keychain_secrets_with_warnings(config, &mut load_profile, push_secret_warning);
}

fn load_legacy_os_keychain_secrets_with_warnings(
    config: &mut KeroseneConfig,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
    mut push_warning: impl FnMut(String),
) {
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
