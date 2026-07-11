use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use zeroize::{Zeroize, Zeroizing};

// Legacy readers fetch only empty targets. This non-secret guard preserves the
// existing field-read decision without handing a canonical value to the
// callback; candidate selection below never persists the guard.
const LEGACY_SECRET_PRESENT_GUARD: &str = "already-resolved";
const LEGACY_HYDROMANCER_CONFLICT: &str = "Multiple legacy Hydromancer API keys were found; choose and save the intended key before switching storage";

// ---------------------------------------------------------------------------
// Secret Storage Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn clear_keychain_credentials_best_effort_with(
        &mut self,
        clear_keychain_secrets: impl FnOnce(&[config::AccountProfile]) -> Result<(), String>,
    ) -> bool {
        let accounts = self.keychain_cleanup_profiles_snapshot();
        if let Err(error) = clear_keychain_secrets(&accounts) {
            self.secret_store_status = Some((
                format!(
                    "Encrypted credentials saved; OS keychain cleanup failed and will retry on next startup: {}",
                    redact_sensitive_response_text(&error)
                ),
                true,
            ));
            false
        } else {
            true
        }
    }

    pub(crate) fn keychain_cleanup_profiles_snapshot(&self) -> Vec<config::AccountProfile> {
        // Cleanup consumers use only secret IDs; keep runtime profile metadata
        // and credentials out of both synchronous and async cleanup owners.
        let mut accounts = self
            .accounts
            .iter()
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            .map(|profile| secret_profile_identity_shell(&profile.secret_id))
            .collect::<Vec<_>>();
        for secret_id in &self.pending_keychain_profile_deletions {
            let secret_id = secret_id.trim();
            if secret_id.is_empty()
                || self.ghost_account_secret_ids.contains(secret_id)
                || accounts
                    .iter()
                    .any(|profile| profile.secret_id == secret_id)
            {
                continue;
            }
            accounts.push(secret_profile_identity_shell(secret_id));
        }
        accounts
    }

    pub(crate) fn apply_secret_storage_selection(&mut self) {
        match self.secret_storage_selection {
            config::CredentialStorageMode::OsKeychain => {
                let (
                    schwab_client_id,
                    schwab_client_secret,
                    schwab_access_token,
                    schwab_refresh_token,
                ) = self.schwab.oauth_credentials_for_secret();
                let openrouter_api_key =
                    Zeroizing::new(self.openrouter_api_key.as_str().to_string());
                self.apply_os_keychain_storage_selection_with(
                    config::save_config,
                    move |profiles,
                          hydromancer_api_key,
                          hyperdash_api_key,
                          x_access_token,
                          x_oauth_client_id,
                          x_refresh_token| {
                        config::store_keychain_secrets_with_profile_removals_with_integrations(
                            profiles,
                            hydromancer_api_key,
                            hyperdash_api_key,
                            x_access_token,
                            x_oauth_client_id,
                            x_refresh_token,
                            schwab_client_id.as_str(),
                            schwab_client_secret.as_str(),
                            schwab_access_token.as_str(),
                            schwab_refresh_token.as_str(),
                            openrouter_api_key.as_str(),
                            &[],
                        )
                    },
                    config::load_keychain_secret_payload,
                    |payload| match payload {
                        Some(payload) => config::store_secret_payload(payload),
                        None => config::clear_keychain_secret_payload(),
                    },
                )
            }
            config::CredentialStorageMode::EncryptedConfig => {
                self.apply_encrypted_config_storage_selection_with(
                    config::save_config,
                    config::clear_all_keychain_secrets,
                    config::load_keychain_secret_payload,
                    config::load_legacy_global_secrets,
                    config::load_legacy_profile_secrets,
                );
            }
        }
    }

    fn apply_os_keychain_storage_selection_with(
        &mut self,
        save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        store_keychain_secrets: impl FnOnce(
            &[config::AccountProfile],
            &str,
            &str,
            &str,
            &str,
            &str,
        ) -> Result<Option<String>, String>,
        load_keychain_secret_payload: impl FnOnce() -> Result<Option<config::SecretPayload>, String>,
        rollback_keychain_secret_payload: impl FnOnce(
            Option<&config::SecretPayload>,
        ) -> Result<(), String>,
    ) {
        if self.secret_storage_mode == config::CredentialStorageMode::EncryptedConfig
            && !self.encrypted_secrets_unlocked
        {
            self.secret_store_status = Some((
                "Unlock encrypted credentials before moving them to the OS keychain".to_string(),
                true,
            ));
            return;
        }

        let previous_mode = self.secret_storage_mode;
        let previous_keychain_payload = (previous_mode
            == config::CredentialStorageMode::EncryptedConfig)
            .then(load_keychain_secret_payload);
        let accounts = self.persisted_accounts_snapshot();
        let (x_access_token, x_oauth_client_id, x_refresh_token) =
            self.x_feed.oauth_credentials_for_secret();
        let cleanup_warning = match store_keychain_secrets(
            &accounts,
            &self.hydromancer_api_key,
            &self.hyperdash_api_key,
            x_access_token.as_str(),
            x_oauth_client_id.as_str(),
            x_refresh_token.as_str(),
        ) {
            Ok(cleanup_warning) => cleanup_warning,
            Err(error) => {
                self.secret_storage_selection = self.secret_storage_mode;
                self.secret_store_status = Some((
                    format!(
                        "OS keychain credential save failed: {}. If OS keychain storage keeps failing, switch to encrypted config in Settings > Storage.",
                        redact_sensitive_response_text(&error)
                    ),
                    true,
                ));
                return;
            }
        };

        let previous_encrypted_secrets = self.encrypted_secrets.clone();
        let previous_unlocked = self.encrypted_secrets_unlocked;
        let previous_save_block = self.secret_migration_save_blocked;
        let previous_pending_cleanup_all = self.pending_keychain_cleanup_all;

        self.secret_migration_save_blocked = false;
        self.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        self.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        self.pending_keychain_cleanup_all = false;
        self.encrypted_secrets = None;
        self.encrypted_secrets_unlocked = false;

        if let Err(error) = self.persist_config_immediately_with(save_config) {
            if config::config_save_installed_snapshot(&error) {
                self.encrypted_secret_password.zeroize();
                self.encrypted_secret_confirm.zeroize();
                self.secret_store_status = Some((
                    Self::committed_config_save_warning("Credentials saved to OS keychain", &error),
                    true,
                ));
                return;
            }
            let mut rollback_warnings = Vec::new();
            if previous_mode == config::CredentialStorageMode::EncryptedConfig {
                let restore_payload = match previous_keychain_payload {
                    Some(Ok(payload)) => payload,
                    Some(Err(error)) => {
                        rollback_warnings.push(format!(
                            "prior OS keychain payload snapshot failed: {}",
                            redact_sensitive_response_text(&error)
                        ));
                        None
                    }
                    None => None,
                };
                if let Err(error) = rollback_keychain_secret_payload(restore_payload.as_ref()) {
                    rollback_warnings.push(format!(
                        "stale OS keychain rollback cleanup failed: {}",
                        redact_sensitive_response_text(&error)
                    ));
                }
            }
            self.secret_storage_mode = previous_mode;
            self.secret_storage_selection = previous_mode;
            self.encrypted_secrets = previous_encrypted_secrets;
            self.encrypted_secrets_unlocked = previous_unlocked;
            self.secret_migration_save_blocked = previous_save_block;
            self.pending_keychain_cleanup_all = previous_pending_cleanup_all;
            let active_storage = match previous_mode {
                config::CredentialStorageMode::EncryptedConfig => "encrypted config",
                config::CredentialStorageMode::OsKeychain => "OS keychain",
            };
            let mut status = format!(
                "OS keychain credential config save failed: {}; {active_storage} credentials remain active",
                redact_sensitive_response_text(&error)
            );
            for warning in rollback_warnings {
                status.push_str("; ");
                status.push_str(&warning);
            }
            self.secret_store_status = Some((status, true));
            return;
        }

        self.encrypted_secret_password.zeroize();
        self.encrypted_secret_confirm.zeroize();
        self.secret_store_status = if let Some(cleanup_warning) = cleanup_warning {
            Some((
                format!(
                    "Credentials saved to OS keychain; legacy cleanup skipped: {}",
                    redact_sensitive_response_text(&cleanup_warning)
                ),
                true,
            ))
        } else {
            Some(("Credentials saved to OS keychain".to_string(), false))
        };
    }

    fn apply_encrypted_config_storage_selection_with(
        &mut self,
        mut save_config: impl FnMut(&config::KeroseneConfig) -> Result<(), String>,
        clear_keychain_secrets: impl FnOnce(&[config::AccountProfile]) -> Result<(), String>,
        load_keychain_secret_payload: impl FnOnce() -> Result<Option<config::SecretPayload>, String>,
        load_global_secrets: impl FnOnce(
            &mut Zeroizing<String>,
            &mut Zeroizing<String>,
        ) -> Result<(), String>,
        load_profile_secrets: impl FnMut(&mut config::AccountProfile) -> Result<(), String>,
    ) {
        if !self.encrypted_password_is_ready() {
            return;
        }
        let confirm_required = self.secret_storage_mode
            != config::CredentialStorageMode::EncryptedConfig
            || self.encrypted_secrets.is_none()
            || !self.encrypted_secret_confirm.is_empty();
        if confirm_required && self.encrypted_secret_password != self.encrypted_secret_confirm {
            self.secret_store_status = Some((
                "Encrypted credential passwords do not match".to_string(),
                true,
            ));
            return;
        }

        let previous_mode = self.secret_storage_mode;
        let previous_encrypted_secrets = self.encrypted_secrets.clone();
        let previous_unlocked = self.encrypted_secrets_unlocked;
        let previous_save_block = self.secret_migration_save_blocked;
        let previous_pending_cleanup_all = self.pending_keychain_cleanup_all;
        let payload = match self.encrypted_storage_selection_payload(
            load_keychain_secret_payload,
            load_global_secrets,
            load_profile_secrets,
        ) {
            Ok(payload) => payload,
            Err(error) => {
                self.secret_storage_selection = self.secret_storage_mode;
                self.secret_store_status = Some((
                    format!(
                        "Encrypted credential migration failed: {}; OS keychain credentials were left unchanged",
                        redact_sensitive_response_text(&error)
                    ),
                    true,
                ));
                return;
            }
        };
        let Some(encrypted) = self.encrypted_secret_blob_for_payload(&payload) else {
            return;
        };

        self.store_encrypted_secret_blob(encrypted, "Credentials saved to encrypted config");
        {
            self.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
            self.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
            self.pending_keychain_cleanup_all =
                previous_mode == config::CredentialStorageMode::OsKeychain;
            self.encrypted_secret_confirm.zeroize();

            self.secret_migration_save_blocked = false;
            if let Err(error) = self.persist_config_immediately_with(&mut save_config) {
                if config::config_save_installed_snapshot(&error) {
                    self.secret_store_status = Some((
                        Self::committed_config_save_warning(
                            "Credentials saved to encrypted config; OS keychain cleanup was deferred",
                            &error,
                        ),
                        true,
                    ));
                    return;
                }
                self.secret_storage_mode = previous_mode;
                self.secret_storage_selection = previous_mode;
                self.encrypted_secrets = previous_encrypted_secrets;
                self.encrypted_secrets_unlocked = previous_unlocked;
                self.secret_migration_save_blocked = previous_save_block;
                self.pending_keychain_cleanup_all = previous_pending_cleanup_all;
                self.secret_store_status = Some((
                    format!(
                        "Encrypted credential config save failed: {}; OS keychain credentials were left unchanged",
                        redact_sensitive_response_text(&error)
                    ),
                    true,
                ));
                return;
            }

            self.secret_migration_save_blocked = false;
            if previous_mode == config::CredentialStorageMode::OsKeychain
                && self.clear_keychain_credentials_best_effort_with(clear_keychain_secrets)
            {
                self.pending_keychain_cleanup_all = false;
                self.pending_keychain_profile_deletions.clear();
                if let Err(error) = self.persist_config_immediately_with(&mut save_config) {
                    self.secret_store_status = Some((
                        format!(
                            "Encrypted credentials saved, but keychain cleanup state save failed: {}",
                            redact_sensitive_response_text(&error)
                        ),
                        true,
                    ));
                }
            }
        }
    }

    fn encrypted_storage_selection_payload(
        &self,
        load_keychain_secret_payload: impl FnOnce() -> Result<Option<config::SecretPayload>, String>,
        load_global_secrets: impl FnOnce(
            &mut Zeroizing<String>,
            &mut Zeroizing<String>,
        ) -> Result<(), String>,
        mut load_profile_secrets: impl FnMut(&mut config::AccountProfile) -> Result<(), String>,
    ) -> Result<config::SecretPayload, String> {
        let mut payload = self.current_secret_payload();

        if self.secret_storage_mode != config::CredentialStorageMode::OsKeychain {
            return Ok(payload);
        }

        let keychain_payload = load_keychain_secret_payload()
            .map_err(|_| "OS keychain credentials could not be read".to_string())?;
        if let Some(keychain_payload) = keychain_payload.as_ref() {
            merge_missing_keychain_payload_secrets(
                &mut payload,
                keychain_payload,
                self.accounts
                    .iter()
                    .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id)),
            );
        }

        let mut hydromancer_api_key =
            legacy_secret_lookup_buffer(payload.global_hydromancer_api_key());
        let mut hyperdash_api_key = legacy_secret_lookup_buffer(payload.global_hyperdash_api_key());
        load_global_secrets(&mut hydromancer_api_key, &mut hyperdash_api_key)
            .map_err(|_| "OS keychain shared credentials could not be read".to_string())?;
        merge_missing_legacy_global_secrets(&mut payload, hydromancer_api_key, hyperdash_api_key);

        for account in self
            .accounts
            .iter()
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
        {
            let secret_id = account.secret_id.trim().to_string();
            if secret_id.is_empty() {
                continue;
            }

            let missing_agent_key = payload
                .profile_agent_key_for_wallet(&secret_id, &account.wallet_address)
                .is_none();
            if missing_agent_key
                && keychain_payload.as_ref().is_some_and(|payload| {
                    payload
                        .profile_agent_key_binding_mismatches(&secret_id, &account.wallet_address)
                })
            {
                return Err(
                    "An OS keychain account key is bound to a different wallet address; re-enter and save that account key before switching storage"
                        .to_string(),
                );
            }

            let mut legacy_profile = secret_profile_identity_shell(&account.secret_id);
            legacy_profile.agent_key = legacy_secret_lookup_buffer(&account.agent_key);
            legacy_profile.hydromancer_api_key =
                legacy_secret_lookup_buffer(&account.hydromancer_api_key);
            load_profile_secrets(&mut legacy_profile)
                .map_err(|_| "OS keychain profile credentials could not be read".to_string())?;
            if missing_agent_key {
                if account.agent_key.trim().is_empty() {
                    let agent_key = std::mem::take(&mut legacy_profile.agent_key);
                    if !agent_key.trim().is_empty() {
                        payload.upsert_profile_agent_key_for_wallet_owned(
                            &secret_id,
                            Some(&account.wallet_address),
                            agent_key,
                        );
                    }
                } else {
                    payload.upsert_profile_agent_key_for_wallet(
                        &secret_id,
                        Some(&account.wallet_address),
                        &account.agent_key,
                    );
                }
            }
            merge_legacy_profile_hydromancer_key(
                &mut payload,
                &account.hydromancer_api_key,
                std::mem::take(&mut legacy_profile.hydromancer_api_key),
            )?;
        }

        Ok(payload)
    }
}

fn secret_profile_identity_shell(secret_id: &str) -> config::AccountProfile {
    config::AccountProfile {
        secret_id: secret_id.to_string(),
        name: String::new(),
        wallet_address: String::new(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    }
}

fn legacy_secret_lookup_buffer(current_value: &str) -> Zeroizing<String> {
    if current_value.trim().is_empty() {
        Zeroizing::new(String::new())
    } else {
        Zeroizing::new(LEGACY_SECRET_PRESENT_GUARD.to_string())
    }
}

fn merge_missing_legacy_global_secrets(
    payload: &mut config::SecretPayload,
    hydromancer_api_key: Zeroizing<String>,
    hyperdash_api_key: Zeroizing<String>,
) {
    if payload.global_hydromancer_api_key().trim().is_empty()
        && !hydromancer_api_key.trim().is_empty()
    {
        payload.set_global_hydromancer_api_key_owned(hydromancer_api_key);
    }
    if payload.global_hyperdash_api_key().trim().is_empty() && !hyperdash_api_key.trim().is_empty()
    {
        payload.set_global_hyperdash_api_key_owned(hyperdash_api_key);
    }
}

fn merge_legacy_profile_hydromancer_key(
    payload: &mut config::SecretPayload,
    current_profile_key: &str,
    loaded_profile_key: Zeroizing<String>,
) -> Result<(), String> {
    let current_profile_key = current_profile_key.trim();
    if !current_profile_key.is_empty() {
        return merge_borrowed_legacy_profile_hydromancer_key(payload, current_profile_key);
    }

    let loaded_profile_key = trim_owned_legacy_profile_hydromancer_key(loaded_profile_key);
    if loaded_profile_key.is_empty() {
        return Ok(());
    }
    let global_hydromancer_key = payload.global_hydromancer_api_key().trim();
    if global_hydromancer_key.is_empty() {
        payload.set_global_hydromancer_api_key_owned(loaded_profile_key);
        Ok(())
    } else if global_hydromancer_key == loaded_profile_key.as_str() {
        Ok(())
    } else {
        Err(LEGACY_HYDROMANCER_CONFLICT.to_string())
    }
}

fn merge_borrowed_legacy_profile_hydromancer_key(
    payload: &mut config::SecretPayload,
    profile_hydromancer_key: &str,
) -> Result<(), String> {
    let global_hydromancer_key = payload.global_hydromancer_api_key().trim();
    if global_hydromancer_key.is_empty() {
        payload.set_global_hydromancer_api_key(profile_hydromancer_key);
        Ok(())
    } else if global_hydromancer_key == profile_hydromancer_key {
        Ok(())
    } else {
        Err(LEGACY_HYDROMANCER_CONFLICT.to_string())
    }
}

fn trim_owned_legacy_profile_hydromancer_key(
    profile_hydromancer_key: Zeroizing<String>,
) -> Zeroizing<String> {
    let trimmed_len = profile_hydromancer_key.trim().len();
    if trimmed_len == profile_hydromancer_key.len() {
        profile_hydromancer_key
    } else {
        Zeroizing::new(profile_hydromancer_key.trim().to_string())
    }
}

fn merge_missing_keychain_payload_secrets<'a>(
    payload: &mut config::SecretPayload,
    keychain_payload: &config::SecretPayload,
    accounts: impl IntoIterator<Item = &'a config::AccountProfile>,
) {
    for account in accounts {
        let secret_id = account.secret_id.trim();
        if secret_id.is_empty()
            || payload
                .profile_agent_key_for_wallet(secret_id, &account.wallet_address)
                .is_some()
        {
            continue;
        }

        if let Some(agent_key) =
            keychain_payload.profile_agent_key_for_wallet(secret_id, &account.wallet_address)
        {
            payload.upsert_profile_agent_key_for_wallet(
                secret_id,
                Some(&account.wallet_address),
                agent_key,
            );
        }
    }

    if payload.global_hydromancer_api_key().trim().is_empty()
        && !keychain_payload
            .global_hydromancer_api_key()
            .trim()
            .is_empty()
    {
        payload.set_global_hydromancer_api_key(keychain_payload.global_hydromancer_api_key());
    }
    if payload.global_hyperdash_api_key().trim().is_empty()
        && !keychain_payload
            .global_hyperdash_api_key()
            .trim()
            .is_empty()
    {
        payload.set_global_hyperdash_api_key(keychain_payload.global_hyperdash_api_key());
    }
    if payload.global_x_access_token().trim().is_empty()
        && !keychain_payload.global_x_access_token().trim().is_empty()
    {
        payload.set_global_x_access_token(keychain_payload.global_x_access_token());
    }
    if payload.global_x_oauth_client_id().trim().is_empty()
        && !keychain_payload
            .global_x_oauth_client_id()
            .trim()
            .is_empty()
    {
        payload.set_global_x_oauth_client_id(keychain_payload.global_x_oauth_client_id());
    }
    if payload.global_x_refresh_token().trim().is_empty()
        && !keychain_payload.global_x_refresh_token().trim().is_empty()
    {
        payload.set_global_x_refresh_token(keychain_payload.global_x_refresh_token());
    }
    if payload.global_schwab_client_id().trim().is_empty()
        && !keychain_payload.global_schwab_client_id().trim().is_empty()
    {
        payload.set_global_schwab_client_id(keychain_payload.global_schwab_client_id());
    }
    if payload.global_schwab_client_secret().trim().is_empty()
        && !keychain_payload
            .global_schwab_client_secret()
            .trim()
            .is_empty()
    {
        payload.set_global_schwab_client_secret(keychain_payload.global_schwab_client_secret());
    }
    if payload.global_schwab_access_token().trim().is_empty()
        && !keychain_payload
            .global_schwab_access_token()
            .trim()
            .is_empty()
    {
        payload.set_global_schwab_access_token(keychain_payload.global_schwab_access_token());
    }
    if payload.global_schwab_refresh_token().trim().is_empty()
        && !keychain_payload
            .global_schwab_refresh_token()
            .trim()
            .is_empty()
    {
        payload.set_global_schwab_refresh_token(keychain_payload.global_schwab_refresh_token());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::sensitive_string;
    use std::cell::{Cell, RefCell};

    fn account(secret_id: &str, agent_key: &str) -> config::AccountProfile {
        config::AccountProfile {
            secret_id: secret_id.to_string(),
            name: secret_id.to_string(),
            wallet_address: "0x0000000000000000000000000000000000000001".to_string(),
            agent_key: sensitive_string(agent_key).into_zeroizing(),
            hydromancer_api_key: sensitive_string("").into_zeroizing(),
        }
    }

    fn terminal_ready_to_switch_to_encrypted() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", "agent-a")];
        terminal.active_account_index = 0;
        terminal.last_persisted_active_account_secret_id = Some("acct-a".to_string());
        terminal.wallet_key_input = sensitive_string("agent-a");
        terminal.hydromancer_api_key = sensitive_string("hydro-a");
        terminal.hyperdash_api_key = sensitive_string("hyper-a");
        terminal
            .x_feed
            .set_oauth_credentials_from_secret("x-a", "x-client-a", "x-refresh-a", None);
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::EncryptedConfig;
        terminal.encrypted_secrets = None;
        terminal.encrypted_secrets_unlocked = false;
        terminal.encrypted_secret_password = sensitive_string("correct horse");
        terminal.encrypted_secret_confirm = sensitive_string("correct horse");
        terminal
    }

    fn terminal_ready_to_switch_to_os_keychain() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![account("acct-a", "agent-a")];
        terminal.active_account_index = 0;
        terminal.last_persisted_active_account_secret_id = Some("acct-a".to_string());
        terminal.wallet_key_input = sensitive_string("agent-a");
        terminal.hydromancer_api_key = sensitive_string("hydro-a");
        terminal.hyperdash_api_key = sensitive_string("hyper-a");
        terminal
            .x_feed
            .set_oauth_credentials_from_secret("x-a", "x-client-a", "x-refresh-a", None);
        terminal.secret_storage_mode = config::CredentialStorageMode::EncryptedConfig;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.encrypted_secret_password = sensitive_string("correct horse");
        terminal.encrypted_secret_confirm = sensitive_string("correct horse");
        let payload = terminal.current_secret_payload();
        terminal.encrypted_secrets =
            Some(config::encrypt_secrets(&payload, "correct horse").expect("encrypt fixture"));
        terminal.encrypted_secrets_unlocked = true;
        terminal
    }

    fn no_keychain_payload() -> Result<Option<config::SecretPayload>, String> {
        Ok(None)
    }

    fn no_legacy_global_secret(
        _hydromancer_api_key: &mut Zeroizing<String>,
        _hyperdash_api_key: &mut Zeroizing<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    fn no_legacy_profile_secret(_profile: &mut config::AccountProfile) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn encrypted_storage_selection_legacy_readers_receive_only_presence_guards() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts[0].hydromancer_api_key = sensitive_string("hydro-a").into_zeroizing();
        terminal.accounts.push(account("ghost-acct", "ghost-agent"));
        terminal
            .ghost_account_secret_ids
            .insert("ghost-acct".to_string());
        let global_reads = Cell::new(0_u32);
        let profile_reads = RefCell::new(Vec::new());

        let payload = terminal
            .encrypted_storage_selection_payload(
                no_keychain_payload,
                |hydromancer_api_key, hyperdash_api_key| {
                    global_reads.set(global_reads.get().saturating_add(1));
                    assert!(!hydromancer_api_key.trim().is_empty());
                    assert!(!hyperdash_api_key.trim().is_empty());
                    assert_ne!(hydromancer_api_key.as_str(), "hydro-a");
                    assert_ne!(hyperdash_api_key.as_str(), "hyper-a");
                    Ok(())
                },
                |profile| {
                    profile_reads.borrow_mut().push(profile.secret_id.clone());
                    assert_eq!(profile.secret_id, "acct-a");
                    assert!(profile.name.is_empty());
                    assert!(profile.wallet_address.is_empty());
                    assert!(!profile.agent_key.trim().is_empty());
                    assert!(!profile.hydromancer_api_key.trim().is_empty());
                    assert_ne!(profile.agent_key.as_str(), "agent-a");
                    assert_ne!(profile.hydromancer_api_key.as_str(), "hydro-a");
                    Ok(())
                },
            )
            .expect("resolved storage selection should preserve canonical credentials");

        assert_eq!(global_reads.get(), 1);
        assert_eq!(profile_reads.borrow().as_slice(), ["acct-a"]);
        assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
        assert_eq!(payload.profile_agent_key("ghost-acct"), None);
        assert_eq!(payload.global_hydromancer_api_key(), "hydro-a");
        assert_eq!(payload.global_hyperdash_api_key(), "hyper-a");
    }

    #[test]
    fn encrypted_storage_selection_moves_loaded_legacy_allocations_into_payload() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
        terminal.hydromancer_api_key = sensitive_string("");
        terminal.hyperdash_api_key = sensitive_string("");
        let loaded_agent_allocation = Cell::new(std::ptr::null::<u8>());
        let loaded_hydromancer_allocation = Cell::new(std::ptr::null::<u8>());
        let loaded_hyperdash_allocation = Cell::new(std::ptr::null::<u8>());

        let payload = terminal
            .encrypted_storage_selection_payload(
                no_keychain_payload,
                |hydromancer_api_key, hyperdash_api_key| {
                    assert!(hydromancer_api_key.is_empty());
                    assert!(hyperdash_api_key.is_empty());
                    *hyperdash_api_key = sensitive_string("legacy-hyper").into_zeroizing();
                    loaded_hyperdash_allocation.set(hyperdash_api_key.as_ptr());
                    Ok(())
                },
                |profile| {
                    profile.agent_key = sensitive_string("legacy-agent").into_zeroizing();
                    profile.hydromancer_api_key = sensitive_string("legacy-hydro").into_zeroizing();
                    loaded_agent_allocation.set(profile.agent_key.as_ptr());
                    loaded_hydromancer_allocation.set(profile.hydromancer_api_key.as_ptr());
                    Ok(())
                },
            )
            .expect("legacy credentials should complete the encrypted payload");

        assert_eq!(
            payload
                .profile_agent_key("acct-a")
                .expect("legacy agent should be retained")
                .as_ptr(),
            loaded_agent_allocation.get()
        );
        assert_eq!(
            payload.global_hydromancer_api_key().as_ptr(),
            loaded_hydromancer_allocation.get()
        );
        assert_eq!(
            payload.global_hyperdash_api_key().as_ptr(),
            loaded_hyperdash_allocation.get()
        );
        assert!(terminal.accounts[0].agent_key.is_empty());
        assert!(terminal.hydromancer_api_key.is_empty());
        assert!(terminal.hyperdash_api_key.is_empty());
    }

    #[test]
    fn encrypted_storage_selection_preserves_legacy_profile_hydromancer_trimming() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.hydromancer_api_key = sensitive_string("");

        let payload = terminal
            .encrypted_storage_selection_payload(
                no_keychain_payload,
                no_legacy_global_secret,
                |profile| {
                    profile.hydromancer_api_key =
                        sensitive_string("  legacy-hydro\n").into_zeroizing();
                    Ok(())
                },
            )
            .expect("legacy Hydromancer whitespace should retain established normalization");

        assert_eq!(payload.global_hydromancer_api_key(), "legacy-hydro");
    }

    #[test]
    fn encrypted_storage_selection_keeps_bundle_agent_authoritative_over_legacy_fallback() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
        let keychain_payload =
            config::SecretPayload::from_credentials(&[account("acct-a", "bundle-agent")], "", "");
        let profile_reads = Cell::new(0_u32);

        let payload = terminal
            .encrypted_storage_selection_payload(
                || Ok(Some(keychain_payload.clone())),
                no_legacy_global_secret,
                |profile| {
                    profile_reads.set(profile_reads.get().saturating_add(1));
                    assert!(profile.agent_key.is_empty());
                    profile.agent_key = sensitive_string("legacy-agent").into_zeroizing();
                    Ok(())
                },
            )
            .expect("the existing bundle should retain its established authority");

        assert_eq!(profile_reads.get(), 1);
        assert_eq!(payload.profile_agent_key("acct-a"), Some("bundle-agent"));
        assert!(terminal.accounts[0].agent_key.is_empty());
    }

    #[test]
    fn keychain_cleanup_profiles_snapshot_includes_pending_deleted_profiles_once() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts.push(account("acct-b", "agent-b"));
        terminal.accounts.push(account("ghost-acct", "ghost-agent"));
        terminal
            .ghost_account_secret_ids
            .insert("ghost-acct".to_string());
        terminal.pending_keychain_profile_deletions = vec![
            " ".to_string(),
            "acct-a".to_string(),
            "acct-deleted".to_string(),
            " acct-deleted ".to_string(),
            "ghost-acct".to_string(),
        ];

        let profiles = terminal.keychain_cleanup_profiles_snapshot();

        assert_eq!(
            profiles
                .iter()
                .map(|profile| profile.secret_id.as_str())
                .collect::<Vec<_>>(),
            ["acct-a", "acct-b", "acct-deleted"]
        );
        for profile in profiles {
            assert!(profile.name.is_empty());
            assert!(profile.wallet_address.is_empty());
            assert!(profile.agent_key.is_empty());
            assert!(profile.hydromancer_api_key.is_empty());
        }
    }

    #[test]
    fn os_keychain_storage_switch_save_failure_keeps_encrypted_config_active() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();
        terminal.secret_migration_save_blocked = true;
        let original_encrypted = terminal.encrypted_secrets.clone();
        let keychain_called = Cell::new(false);
        let cleanup_called = Cell::new(false);
        let previous_keychain_payload =
            config::SecretPayload::from_credentials(&[account("stale", "stale-agent")], "", "");
        let expected_keychain_payload = previous_keychain_payload.clone();
        let restored_keychain_payload = RefCell::new(None);

        terminal.apply_os_keychain_storage_selection_with(
            |_| Err("disk full".to_string()),
            |profiles,
             hydromancer_key,
             hyperdash_key,
             x_access_token,
             x_oauth_client_id,
             x_refresh_token| {
                keychain_called.set(true);
                assert_eq!(profiles.len(), 1);
                assert_eq!(profiles[0].secret_id, "acct-a");
                assert_eq!(hydromancer_key, "hydro-a");
                assert_eq!(hyperdash_key, "hyper-a");
                assert_eq!(x_access_token, "x-a");
                assert_eq!(x_oauth_client_id, "x-client-a");
                assert_eq!(x_refresh_token, "x-refresh-a");
                Ok(None)
            },
            || Ok(Some(previous_keychain_payload)),
            |payload| {
                cleanup_called.set(true);
                restored_keychain_payload.replace(payload.cloned());
                Ok(())
            },
        );

        assert!(keychain_called.get());
        assert!(cleanup_called.get());
        assert_eq!(
            *restored_keychain_payload.borrow(),
            Some(expected_keychain_payload)
        );
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert_eq!(terminal.encrypted_secrets, original_encrypted);
        assert!(terminal.encrypted_secrets_unlocked);
        assert_eq!(terminal.encrypted_secret_password.as_str(), "correct horse");
        assert!(terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("disk full"));
        assert!(status.contains("encrypted config credentials remain active"));
        assert!(!status.contains("Credentials saved to OS keychain"));
    }

    #[test]
    fn os_keychain_storage_switch_post_commit_warning_keeps_os_keychain_active() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();
        let keychain_called = Cell::new(false);
        let rollback_called = Cell::new(false);

        terminal.apply_os_keychain_storage_selection_with(
            |_| {
                Err(config::installed_config_save_error_for_test(
                    "sync denied signature=os-post-secret",
                ))
            },
            |_, _, _, _, _, _| {
                keychain_called.set(true);
                Ok(None)
            },
            || Ok(None),
            |_| {
                rollback_called.set(true);
                Ok(())
            },
        );

        assert!(keychain_called.get());
        assert!(!rollback_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::OsKeychain
        );
        assert!(terminal.encrypted_secrets.is_none());
        assert!(!terminal.encrypted_secrets_unlocked);
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.encrypted_secret_password.is_empty());
        assert!(terminal.config_save_due_at.is_none());
        let (status, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(status.contains("Credentials saved to OS keychain"));
        assert!(status.contains("config durability could not be fully verified"));
        assert!(status.contains("signature=<redacted>"));
        assert!(!status.contains("os-post-secret"));
        assert!(!status.contains("remain active"));
    }

    #[test]
    fn os_keychain_storage_switch_saves_snapshot_before_reporting_success() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();
        let saved_snapshot = RefCell::new(None);
        let keychain_called = Cell::new(false);

        terminal.apply_os_keychain_storage_selection_with(
            |snapshot| {
                saved_snapshot.replace(Some(snapshot.clone()));
                Ok(())
            },
            |profiles,
             hydromancer_key,
             hyperdash_key,
             x_access_token,
             x_oauth_client_id,
             x_refresh_token| {
                keychain_called.set(true);
                assert_eq!(profiles.len(), 1);
                assert_eq!(profiles[0].secret_id, "acct-a");
                assert_eq!(hydromancer_key, "hydro-a");
                assert_eq!(hyperdash_key, "hyper-a");
                assert_eq!(x_access_token, "x-a");
                assert_eq!(x_oauth_client_id, "x-client-a");
                assert_eq!(x_refresh_token, "x-refresh-a");
                Ok(None)
            },
            || Ok(None),
            |_| panic!("rollback cleanup should not run after a successful config save"),
        );

        assert!(keychain_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::OsKeychain
        );
        assert!(terminal.encrypted_secrets.is_none());
        assert!(!terminal.encrypted_secrets_unlocked);
        assert!(terminal.encrypted_secret_password.is_empty());
        assert!(terminal.encrypted_secret_confirm.is_empty());
        assert!(terminal.config_save_due_at.is_none());
        assert_eq!(
            terminal.secret_store_status,
            Some(("Credentials saved to OS keychain".to_string(), false))
        );

        let snapshot = saved_snapshot
            .borrow()
            .clone()
            .expect("OS keychain snapshot should be saved before success");
        assert_eq!(
            snapshot.credential_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        assert!(snapshot.encrypted_secrets.is_none());
        assert_eq!(snapshot.accounts.len(), 1);
        assert_eq!(snapshot.accounts[0].secret_id, "acct-a");
        assert!(snapshot.accounts[0].agent_key.is_empty());
        assert!(snapshot.accounts[0].hydromancer_api_key.is_empty());
    }

    #[test]
    fn os_keychain_storage_switch_can_recover_from_secret_migration_save_block() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();
        terminal.secret_migration_save_blocked = true;
        let save_called = Cell::new(false);

        terminal.apply_os_keychain_storage_selection_with(
            |_| {
                save_called.set(true);
                Ok(())
            },
            |_, _, _, _, _, _| Ok(None),
            || Ok(None),
            |_| panic!("rollback cleanup should not run after a successful config save"),
        );

        assert!(save_called.get());
        assert!(!terminal.secret_migration_save_blocked);
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
    }

    #[test]
    fn os_keychain_storage_switch_save_failure_reports_rollback_cleanup_failure() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();

        terminal.apply_os_keychain_storage_selection_with(
            |_| Err("disk full api_key=save-secret".to_string()),
            |_, _, _, _, _, _| Ok(None),
            || Ok(None),
            |_| Err("access denied auth_token=rollback-secret".to_string()),
        );

        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("disk full"));
        assert!(status.contains("api_key=<redacted>"));
        assert!(!status.contains("save-secret"));
        assert!(status.contains("encrypted config credentials remain active"));
        assert!(status.contains("stale OS keychain rollback cleanup failed: access denied"));
        assert!(status.contains("auth_token=<redacted>"));
        assert!(!status.contains("rollback-secret"));
    }

    #[test]
    fn os_keychain_storage_switch_save_failure_reports_snapshot_failure() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();
        let cleanup_called = Cell::new(false);

        terminal.apply_os_keychain_storage_selection_with(
            |_| Err("disk full".to_string()),
            |_, _, _, _, _, _| Ok(None),
            || Err("keychain read failed client_secret=read-secret".to_string()),
            |payload| {
                cleanup_called.set(true);
                assert!(payload.is_none());
                Ok(())
            },
        );

        assert!(cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("disk full"));
        assert!(status.contains("prior OS keychain payload snapshot failed: keychain read failed"));
        assert!(status.contains("client_secret=<redacted>"));
        assert!(!status.contains("read-secret"));
        assert!(!status.contains("rollback cleanup failed"));
    }

    #[test]
    fn os_keychain_storage_save_failure_keeps_active_keychain_payload() {
        let mut terminal = terminal_ready_to_switch_to_os_keychain();
        terminal.secret_storage_mode = config::CredentialStorageMode::OsKeychain;
        terminal.secret_storage_selection = config::CredentialStorageMode::OsKeychain;
        terminal.encrypted_secrets = None;
        terminal.encrypted_secrets_unlocked = false;
        let cleanup_called = Cell::new(false);

        terminal.apply_os_keychain_storage_selection_with(
            |_| Err("disk full signature=active-secret".to_string()),
            |_, _, _, _, _, _| Ok(None),
            || panic!("snapshot should not run while OS keychain is already active"),
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
        );

        assert!(!cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("disk full"));
        assert!(status.contains("signature=<redacted>"));
        assert!(!status.contains("active-secret"));
        assert!(status.contains("OS keychain credentials remain active"));
        assert!(!status.contains("encrypted config credentials remain active"));
        assert!(!status.contains("rollback cleanup failed"));
    }

    #[test]
    fn encrypted_storage_switch_save_failure_keeps_keychain_and_rolls_back_mode() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        let cleanup_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |_| Err("disk full api_key=encrypted-switch-secret".to_string()),
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert!(!cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::OsKeychain
        );
        assert!(terminal.encrypted_secrets.is_none());
        assert!(!terminal.encrypted_secrets_unlocked);
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("disk full"));
        assert!(status.contains("api_key=<redacted>"));
        assert!(!status.contains("encrypted-switch-secret"));
        assert!(status.contains("left unchanged"));
    }

    #[test]
    fn encrypted_storage_switch_post_commit_warning_keeps_encrypted_mode_without_cleanup() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        let cleanup_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |_| {
                Err(config::installed_config_save_error_for_test(
                    "sync denied client_secret=deferred-secret",
                ))
            },
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert!(!cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert!(terminal.encrypted_secrets.is_some());
        assert!(terminal.encrypted_secrets_unlocked);
        assert!(terminal.pending_keychain_cleanup_all);
        assert!(!terminal.secret_migration_save_blocked);
        assert!(terminal.config_save_due_at.is_none());
        let (status, is_error) = terminal.secret_store_status.as_ref().expect("status");
        assert!(*is_error);
        assert!(status.contains("Credentials saved to encrypted config"));
        assert!(status.contains("OS keychain cleanup was deferred"));
        assert!(status.contains("config durability could not be fully verified"));
        assert!(status.contains("client_secret=<redacted>"));
        assert!(!status.contains("deferred-secret"));
        assert!(!status.contains("left unchanged"));
    }

    #[test]
    fn encrypted_storage_switch_saves_encrypted_snapshot_before_keychain_cleanup() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        let saved_snapshots = RefCell::new(Vec::new());
        let cleanup_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |snapshot| {
                saved_snapshots.borrow_mut().push(snapshot.clone());
                Ok(())
            },
            |profiles| {
                let snapshots = saved_snapshots.borrow();
                assert_eq!(snapshots.len(), 1);
                assert!(snapshots[0].pending_keychain_cleanup_all);
                cleanup_called.set(true);
                assert_eq!(profiles.len(), 1);
                assert_eq!(profiles[0].secret_id, "acct-a");
                Ok(())
            },
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert!(cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert!(!terminal.pending_keychain_cleanup_all);
        let snapshots = saved_snapshots.borrow();
        assert_eq!(snapshots.len(), 2);
        let snapshot = &snapshots[0];
        assert_eq!(
            snapshot.credential_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        assert!(snapshot.pending_keychain_cleanup_all);
        assert!(snapshot.encrypted_secrets.is_some());
        assert_eq!(snapshot.accounts.len(), 1);
        assert_eq!(snapshot.accounts[0].secret_id, "acct-a");
        assert!(snapshot.accounts[0].agent_key.is_empty());
        assert!(snapshot.accounts[0].hydromancer_api_key.is_empty());
        let payload = config::decrypt_secrets(
            snapshot
                .encrypted_secrets
                .as_ref()
                .expect("encrypted payload should be saved"),
            "correct horse",
        )
        .expect("encrypted payload should decrypt");
        assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
        assert_eq!(payload.global_x_access_token(), "x-a");
        assert_eq!(payload.global_x_oauth_client_id(), "x-client-a");
        assert_eq!(payload.global_x_refresh_token(), "x-refresh-a");
        assert_eq!(snapshot.hydromancer_api_key.as_str(), "");
        assert_eq!(snapshot.hyperdash_api_key.as_str(), "");
        assert!(!snapshots[1].pending_keychain_cleanup_all);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn encrypted_storage_switch_cleanup_failure_keeps_full_keychain_retry_intent() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        let saved_snapshots = RefCell::new(Vec::new());

        terminal.apply_encrypted_config_storage_selection_with(
            |snapshot| {
                saved_snapshots.borrow_mut().push(snapshot.clone());
                Ok(())
            },
            |_| Err("keychain denied auth_token=cleanup-secret".to_string()),
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert!(terminal.pending_keychain_cleanup_all);
        let saved_snapshots = saved_snapshots.borrow();
        assert_eq!(saved_snapshots.len(), 1);
        assert!(saved_snapshots[0].pending_keychain_cleanup_all);
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("cleanup failure should set status");
        assert!(*is_error);
        assert!(status.contains("will retry on next startup"));
        assert!(status.contains("auth_token=<redacted>"));
        assert!(!status.contains("cleanup-secret"));
    }

    #[test]
    fn encrypted_storage_switch_cleans_pending_keychain_profiles_and_saves_cleared_intent() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal
            .pending_keychain_profile_deletions
            .push("acct-deleted".to_string());
        let saved_snapshots = RefCell::new(Vec::new());
        let cleaned_profiles = RefCell::new(Vec::new());

        terminal.apply_encrypted_config_storage_selection_with(
            |snapshot| {
                saved_snapshots.borrow_mut().push(snapshot.clone());
                Ok(())
            },
            |profiles| {
                cleaned_profiles.replace(
                    profiles
                        .iter()
                        .map(|profile| profile.secret_id.clone())
                        .collect(),
                );
                Ok(())
            },
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert!(terminal.pending_keychain_profile_deletions.is_empty());
        assert!(!terminal.pending_keychain_cleanup_all);
        assert_eq!(
            cleaned_profiles.borrow().as_slice(),
            ["acct-a".to_string(), "acct-deleted".to_string()]
        );
        let saved_snapshots = saved_snapshots.borrow();
        assert_eq!(saved_snapshots.len(), 2);
        assert_eq!(
            saved_snapshots[0]
                .pending_keychain_profile_deletions
                .as_slice(),
            ["acct-deleted"]
        );
        assert!(saved_snapshots[0].pending_keychain_cleanup_all);
        assert!(
            saved_snapshots[1]
                .pending_keychain_profile_deletions
                .is_empty()
        );
        assert!(!saved_snapshots[1].pending_keychain_cleanup_all);
    }

    #[test]
    fn encrypted_storage_switch_hydrates_deferred_legacy_profile_before_cleanup() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts.push(account("acct-b", ""));
        terminal.hyperdash_api_key = sensitive_string("");
        let saved_snapshot = RefCell::new(None);
        let cleanup_called = Cell::new(false);
        let loaded_profiles = RefCell::new(Vec::new());
        let keychain_payload = config::SecretPayload::from_credentials(
            &[account("acct-a", "agent-a")],
            "",
            "keychain-hyper",
        );

        terminal.apply_encrypted_config_storage_selection_with(
            |snapshot| {
                saved_snapshot.replace(Some(snapshot.clone()));
                Ok(())
            },
            |profiles| {
                cleanup_called.set(true);
                assert_eq!(
                    profiles
                        .iter()
                        .map(|profile| profile.secret_id.as_str())
                        .collect::<Vec<_>>(),
                    ["acct-a", "acct-b"]
                );
                Ok(())
            },
            || Ok(Some(keychain_payload.clone())),
            no_legacy_global_secret,
            |profile| {
                loaded_profiles.borrow_mut().push(profile.secret_id.clone());
                if profile.secret_id == "acct-b" {
                    profile.agent_key = sensitive_string("agent-b").into_zeroizing();
                }
                Ok(())
            },
        );

        assert!(cleanup_called.get());
        assert_eq!(loaded_profiles.borrow().as_slice(), ["acct-a", "acct-b"]);
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        let snapshot = saved_snapshot
            .borrow()
            .clone()
            .expect("encrypted snapshot should be saved");
        let encrypted = snapshot
            .encrypted_secrets
            .as_ref()
            .expect("encrypted payload should be persisted");
        let payload = config::decrypt_secrets(encrypted, "correct horse")
            .expect("encrypted payload should decrypt");
        assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
        assert_eq!(payload.profile_agent_key("acct-b"), Some("agent-b"));
        assert_eq!(payload.global_hyperdash_api_key(), "keychain-hyper");
    }

    #[test]
    fn encrypted_storage_switch_blocks_when_deferred_legacy_profile_read_fails() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts.push(account("acct-b", ""));
        let save_called = Cell::new(false);
        let cleanup_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |_| {
                save_called.set(true);
                Ok(())
            },
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
            no_keychain_payload,
            no_legacy_global_secret,
            |profile| {
                if profile.secret_id == "acct-b" {
                    return Err("keychain locked".to_string());
                }
                Ok(())
            },
        );

        assert!(!save_called.get());
        assert!(!cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        assert_eq!(
            terminal.secret_storage_selection,
            config::CredentialStorageMode::OsKeychain
        );
        assert!(terminal.encrypted_secrets.is_none());
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("OS keychain profile credentials could not be read"));
        assert!(status.contains("left unchanged"));
    }

    #[test]
    fn encrypted_storage_switch_blocks_legacy_fallback_after_wallet_mismatch() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.accounts[0].wallet_address =
            "0x2222222222222222222222222222222222222222".to_string();
        terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
        let keychain_payload = config::SecretPayload::from_credentials(
            &[config::AccountProfile {
                wallet_address: "0x1111111111111111111111111111111111111111".to_string(),
                ..account("acct-a", "stale-agent")
            }],
            "",
            "",
        );
        let save_called = Cell::new(false);
        let cleanup_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |_| {
                save_called.set(true);
                Ok(())
            },
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
            || Ok(Some(keychain_payload.clone())),
            no_legacy_global_secret,
            |_profile| panic!("mismatched keychain bundle must not fall back to legacy profile"),
        );

        assert!(!save_called.get());
        assert!(!cleanup_called.get());
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::OsKeychain
        );
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("bound to a different wallet address"));
        assert!(status.contains("left unchanged"));
    }

    #[test]
    fn encrypted_storage_switch_hydrates_legacy_integration_keys_before_cleanup() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.hydromancer_api_key = sensitive_string("");
        terminal.hyperdash_api_key = sensitive_string("");
        let saved_snapshot = RefCell::new(None);
        let cleanup_called = Cell::new(false);
        let loaded_profiles = RefCell::new(Vec::new());

        terminal.apply_encrypted_config_storage_selection_with(
            |snapshot| {
                saved_snapshot.replace(Some(snapshot.clone()));
                Ok(())
            },
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
            no_keychain_payload,
            |hydromancer_api_key, hyperdash_api_key| {
                assert!(hydromancer_api_key.trim().is_empty());
                *hyperdash_api_key = sensitive_string("legacy-hyper").into_zeroizing();
                Ok(())
            },
            |profile| {
                loaded_profiles.borrow_mut().push(profile.secret_id.clone());
                if profile.secret_id == "acct-a" {
                    profile.hydromancer_api_key =
                        sensitive_string("legacy-profile-hydro").into_zeroizing();
                }
                Ok(())
            },
        );

        assert!(cleanup_called.get());
        assert_eq!(loaded_profiles.borrow().as_slice(), ["acct-a"]);
        let snapshot = saved_snapshot
            .borrow()
            .clone()
            .expect("encrypted snapshot should be saved");
        let payload = config::decrypt_secrets(
            snapshot
                .encrypted_secrets
                .as_ref()
                .expect("encrypted payload should be saved"),
            "correct horse",
        )
        .expect("encrypted payload should decrypt");
        assert_eq!(payload.global_hydromancer_api_key(), "legacy-profile-hydro");
        assert_eq!(payload.global_hyperdash_api_key(), "legacy-hyper");
    }

    #[test]
    fn encrypted_storage_switch_blocks_conflicting_legacy_profile_hydromancer_keys() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.hydromancer_api_key = sensitive_string("");
        terminal.accounts.push(account("acct-b", "agent-b"));
        let save_called = Cell::new(false);
        let cleanup_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |_| {
                save_called.set(true);
                Ok(())
            },
            |_| {
                cleanup_called.set(true);
                Ok(())
            },
            no_keychain_payload,
            no_legacy_global_secret,
            |profile| {
                profile.hydromancer_api_key =
                    sensitive_string(format!("{}-hydro", profile.secret_id)).into_zeroizing();
                Ok(())
            },
        );

        assert!(!save_called.get());
        assert!(!cleanup_called.get());
        let (status, is_error) = terminal
            .secret_store_status
            .as_ref()
            .expect("failure status should be set");
        assert!(*is_error);
        assert!(status.contains("Multiple legacy Hydromancer API keys"));
        assert!(status.contains("left unchanged"));
    }

    #[test]
    fn encrypted_storage_switch_overwrites_inactive_locked_blob_from_keychain_mode() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        let stale_payload =
            config::SecretPayload::from_credentials(&[account("stale", "stale-agent")], "", "");
        terminal.encrypted_secrets =
            Some(config::encrypt_secrets(&stale_payload, "old password").expect("encrypt stale"));
        terminal.encrypted_secrets_unlocked = false;
        let saved_snapshot = RefCell::new(None);

        terminal.apply_encrypted_config_storage_selection_with(
            |snapshot| {
                saved_snapshot.replace(Some(snapshot.clone()));
                Ok(())
            },
            |_| Ok(()),
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
        let snapshot = saved_snapshot
            .borrow()
            .clone()
            .expect("encrypted snapshot should be saved");
        let payload = config::decrypt_secrets(
            snapshot
                .encrypted_secrets
                .as_ref()
                .expect("new encrypted payload should be saved"),
            "correct horse",
        )
        .expect("new encrypted payload should decrypt");
        assert_eq!(payload.profile_agent_key("acct-a"), Some("agent-a"));
        assert_eq!(payload.profile_agent_key("stale"), None);
    }

    #[test]
    fn encrypted_storage_switch_can_recover_from_secret_migration_save_block() {
        let mut terminal = terminal_ready_to_switch_to_encrypted();
        terminal.secret_migration_save_blocked = true;
        let save_called = Cell::new(false);

        terminal.apply_encrypted_config_storage_selection_with(
            |_| {
                save_called.set(true);
                Ok(())
            },
            |_| Ok(()),
            no_keychain_payload,
            no_legacy_global_secret,
            no_legacy_profile_secret,
        );

        assert!(save_called.get());
        assert!(!terminal.secret_migration_save_blocked);
        assert_eq!(
            terminal.secret_storage_mode,
            config::CredentialStorageMode::EncryptedConfig
        );
    }
}
