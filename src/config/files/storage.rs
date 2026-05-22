use crate::config::secrets::{
    load_keychain_secret_payload, load_profile_secrets, push_secret_warning, store_secret_payload,
};
use crate::config::{
    AccountProfile, CredentialStorageMode, KeroseneConfig, SecretPayload, new_secret_id,
};
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
    load_legacy_os_keychain_secrets_with(config, load_profile_secrets);
}

fn load_legacy_os_keychain_secrets_with(
    config: &mut KeroseneConfig,
    mut load_profile: impl FnMut(&mut AccountProfile) -> Result<(), String>,
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
            push_secret_warning(format!("{}: {error}", profile.name));
        }
    }

    if !config.accounts[active_index].agent_key.trim().is_empty()
        && has_deferred_legacy_keychain_secrets(config, active_index)
    {
        push_secret_warning(
            "Only the active legacy account key was read on startup to avoid repeated macOS Keychain prompts; other legacy account keys will migrate when you switch to them."
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::take_secret_warnings;

    use std::sync::{Mutex, MutexGuard};

    static SECRET_WARNING_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn secret_warning_test_lock() -> MutexGuard<'static, ()> {
        SECRET_WARNING_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn test_profile(name: &str, agent_key: &str) -> AccountProfile {
        AccountProfile {
            secret_id: name.to_string(),
            name: name.to_string(),
            wallet_address: String::new(),
            agent_key: agent_key.to_string().into(),
            hydromancer_api_key: String::new().into(),
        }
    }

    #[test]
    fn legacy_keychain_startup_only_loads_active_missing_agent_key() {
        let _guard = secret_warning_test_lock();
        let _ = take_secret_warnings();
        let mut config = KeroseneConfig {
            active_account_index: 1,
            accounts: vec![test_profile("one", ""), test_profile("two", "")],
            ..KeroseneConfig::default()
        };
        let mut loaded_profiles = Vec::new();

        load_legacy_os_keychain_secrets_with(&mut config, |profile| {
            loaded_profiles.push(profile.name.clone());
            profile.agent_key = format!("{}-agent", profile.name).into();
            Ok(())
        });

        assert_eq!(loaded_profiles, vec!["two"]);
        assert_eq!(config.accounts[0].agent_key.as_str(), "");
        assert_eq!(config.accounts[1].agent_key.as_str(), "two-agent");
        assert!(
            take_secret_warnings()
                .iter()
                .any(|warning| warning.contains("Only the active legacy account key"))
        );
    }

    #[test]
    fn legacy_keychain_startup_skips_keychain_when_active_key_is_plaintext() {
        let _guard = secret_warning_test_lock();
        let _ = take_secret_warnings();
        let mut config = KeroseneConfig {
            active_account_index: 0,
            accounts: vec![test_profile("one", "plain-agent"), test_profile("two", "")],
            ..KeroseneConfig::default()
        };
        let mut load_count = 0;

        load_legacy_os_keychain_secrets_with(&mut config, |_profile| {
            load_count += 1;
            Ok(())
        });

        assert_eq!(load_count, 0);
        assert_eq!(config.accounts[0].agent_key.as_str(), "plain-agent");
        assert!(
            take_secret_warnings()
                .iter()
                .any(|warning| warning.contains("Only the active legacy account key"))
        );
    }

    #[test]
    fn legacy_keychain_startup_preserves_plaintext_integration_keys_without_reads() {
        let _guard = secret_warning_test_lock();
        let _ = take_secret_warnings();
        let mut config = KeroseneConfig {
            accounts: vec![AccountProfile {
                secret_id: "one".to_string(),
                name: "one".to_string(),
                wallet_address: String::new(),
                agent_key: "plain-agent".to_string().into(),
                hydromancer_api_key: "profile-hydro".to_string().into(),
            }],
            hyperdash_api_key: "global-hyper".to_string().into(),
            ..KeroseneConfig::default()
        };

        load_legacy_os_keychain_secrets_with(&mut config, |_profile| {
            panic!("active plaintext agent key should not read the keychain");
        });

        assert_eq!(config.hydromancer_api_key.as_str(), "profile-hydro");
        assert_eq!(config.hyperdash_api_key.as_str(), "global-hyper");
        assert_eq!(config.accounts[0].hydromancer_api_key.as_str(), "");
        assert!(take_secret_warnings().is_empty());
    }
}
