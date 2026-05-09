use crate::config::secrets::{
    clear_profile_hydromancer_secret, load_global_hydromancer_secret, load_global_hyperdash_secret,
    load_profile_secrets, push_secret_warning,
};
use crate::config::{CredentialStorageMode, KeroseneConfig, new_secret_id};
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
    for profile in &mut config.accounts {
        if let Err(error) = load_profile_secrets(profile) {
            push_secret_warning(format!("{}: {error}", profile.name));
        }
    }

    let mut legacy_hydromancer_key = std::mem::take(&mut config.hydromancer_api_key);
    let mut profiles_with_legacy_hydromancer = Vec::new();
    for (index, profile) in config.accounts.iter_mut().enumerate() {
        if !profile.hydromancer_api_key.trim().is_empty() {
            if legacy_hydromancer_key.trim().is_empty() {
                legacy_hydromancer_key = profile.hydromancer_api_key.clone();
            }
            profiles_with_legacy_hydromancer.push(index);
            profile.hydromancer_api_key.zeroize();
        }
    }
    config.hydromancer_api_key =
        load_global_hydromancer_secret(legacy_hydromancer_key.to_string()).into();
    for index in profiles_with_legacy_hydromancer {
        if let Some(profile) = config.accounts.get(index)
            && let Err(error) = clear_profile_hydromancer_secret(profile)
        {
            push_secret_warning(format!(
                "{}: Hydromancer legacy key cleanup failed: {error}",
                profile.name
            ));
        }
    }

    let legacy_hyperdash_key = std::mem::take(&mut config.hyperdash_api_key);
    config.hyperdash_api_key =
        load_global_hyperdash_secret(legacy_hyperdash_key.to_string()).into();
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
