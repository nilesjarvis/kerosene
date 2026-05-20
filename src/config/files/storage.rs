use crate::config::secrets::{
    load_global_hydromancer_secret, load_global_hyperdash_secret, load_keychain_secret_payload,
    load_profile_hydromancer_secret, load_profile_secrets, push_secret_warning,
    store_secret_payload,
};
use crate::config::{CredentialStorageMode, KeroseneConfig, SecretPayload, new_secret_id};
use zeroize::Zeroize;

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
    );
    if !payload.is_empty()
        && let Err(error) = store_secret_payload(&payload)
    {
        push_secret_warning(format!("Credential bundle migration failed: {error}"));
    }
}

fn load_legacy_os_keychain_secrets(config: &mut KeroseneConfig) {
    for profile in &mut config.accounts {
        if let Err(error) = load_profile_secrets(profile) {
            push_secret_warning(format!("{}: {error}", profile.name));
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
    config.hydromancer_api_key =
        load_global_hydromancer_secret(legacy_hydromancer_key.to_string()).into();
    if config.hydromancer_api_key.trim().is_empty() {
        for profile in &config.accounts {
            match load_profile_hydromancer_secret(profile) {
                Ok(Some(secret)) if !secret.trim().is_empty() => {
                    config.hydromancer_api_key = secret.into();
                    break;
                }
                Ok(_) => {}
                Err(error) => push_secret_warning(format!("{}: {error}", profile.name)),
            }
        }
    }

    let legacy_hyperdash_key = std::mem::take(&mut config.hyperdash_api_key);
    config.hyperdash_api_key =
        load_global_hyperdash_secret(legacy_hyperdash_key.to_string()).into();
}

fn merge_missing_plaintext_secrets_into_payload(
    config: &KeroseneConfig,
    payload: &mut SecretPayload,
) -> bool {
    let mut changed = false;

    for profile in &config.accounts {
        if payload
            .profile_agent_key(&profile.secret_id)
            .is_none_or(|agent_key| agent_key.trim().is_empty())
            && !profile.agent_key.trim().is_empty()
        {
            changed |= payload.upsert_profile_agent_key(&profile.secret_id, &profile.agent_key);
        }

        if payload.global_hydromancer_api_key().trim().is_empty()
            && !profile.hydromancer_api_key.trim().is_empty()
        {
            changed |= payload.set_global_hydromancer_api_key(&profile.hydromancer_api_key);
        }
    }

    if payload.global_hydromancer_api_key().trim().is_empty()
        && !config.hydromancer_api_key.trim().is_empty()
    {
        changed |= payload.set_global_hydromancer_api_key(&config.hydromancer_api_key);
    }
    if payload.global_hyperdash_api_key().trim().is_empty()
        && !config.hyperdash_api_key.trim().is_empty()
    {
        changed |= payload.set_global_hyperdash_api_key(&config.hyperdash_api_key);
    }

    changed
}

fn apply_secret_payload(config: &mut KeroseneConfig, payload: &SecretPayload) {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }

        profile.agent_key.zeroize();
        if let Some(agent_key) = payload.profile_agent_key(&profile.secret_id) {
            profile.agent_key = agent_key.to_string().into();
        }
        profile.hydromancer_api_key.zeroize();
    }

    config.hydromancer_api_key.zeroize();
    config.hydromancer_api_key = payload.global_hydromancer_api_key().to_string().into();
    config.hyperdash_api_key.zeroize();
    config.hyperdash_api_key = payload.global_hyperdash_api_key().to_string().into();
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
