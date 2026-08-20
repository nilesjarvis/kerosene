use super::super::{AccountProfile, new_secret_id};
use super::model::{SECRET_PAYLOAD_SCHEMA, SecretPayload, redacted_secret_payload_parse_error};
use crate::config::in_memory_config_mode;
use crate::helpers::redact_sensitive_response_text;
use zeroize::Zeroizing;

#[cfg(target_os = "windows")]
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicU64, Ordering};

const KEYCHAIN_SERVICE: &str = "kerosene";
const GLOBAL_SECRET_ID: &str = "global";
const KEYCHAIN_PAYLOAD_FIELD: &str = "secrets_v1";
#[cfg(target_os = "windows")]
const KEYCHAIN_SHARD_MANIFEST_FIELD: &str = "secrets_v2_manifest";
#[cfg(target_os = "windows")]
const KEYCHAIN_SHARD_FIELD_PREFIX: &str = "secrets_v2_chunk";
#[cfg(target_os = "windows")]
const KEYCHAIN_SHARD_SCHEMA: &str = "kerosene.keychain.shards.v1";
/// Windows Generic Credentials allow 2,560 bytes. `keyring` encodes passwords
/// as UTF-16, so keeping chunks below 1,000 code units leaves ample headroom.
#[cfg(target_os = "windows")]
const KEYCHAIN_SHARD_UTF16_LIMIT: usize = 1_000;
#[cfg(target_os = "windows")]
const KEYCHAIN_MAX_SHARDS: usize = 256;

#[cfg(target_os = "windows")]
static KEYCHAIN_SHARD_GENERATION: AtomicU64 = AtomicU64::new(0);

/// A single, explicit credential mutation.
///
/// Normal credential-entry flows must use this instead of replacing the full
/// bundle from runtime state. That keeps a temporarily unavailable or
/// not-yet-hydrated credential from being interpreted as an explicit clear.
#[derive(Clone, Copy)]
pub(crate) enum KeychainSecretUpdate<'a> {
    Profile(&'a AccountProfile),
    RemoveProfile(&'a str),
    Hydromancer(&'a str),
    Hyperdash(&'a str),
    XOAuth {
        access_token: &'a str,
        client_id: &'a str,
        refresh_token: &'a str,
    },
    SchwabOAuth {
        client_id: &'a str,
        client_secret: &'a str,
        access_token: &'a str,
        refresh_token: &'a str,
    },
    OpenRouter(&'a str),
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize, Deserialize)]
struct KeychainShardManifest {
    schema: String,
    generation: String,
    chunks: usize,
}

fn keychain_account(secret_id: &str, field: &str) -> String {
    format!("{secret_id}:{field}")
}

fn keychain_error_message(
    action: &str,
    secret_id: &str,
    field: &str,
    error: impl std::fmt::Display,
) -> String {
    let error = redacted_keychain_error(secret_id, field, &error.to_string());
    format!("keychain {action} failed: {error}")
}

fn redacted_keychain_error(secret_id: &str, field: &str, error: &str) -> String {
    let account = keychain_account(secret_id, field);
    let error = redact_sensitive_response_text(error);
    let mut redacted = if account.trim().is_empty() {
        error
    } else {
        error.replace(&account, "<keychain-entry>")
    };

    let secret_id = secret_id.trim();
    if !secret_id.is_empty() && secret_id != GLOBAL_SECRET_ID {
        redacted = redacted.replace(secret_id, "<redacted-profile>");
    }

    redacted
}

fn keychain_get(secret_id: &str, field: &str) -> Result<Option<Zeroizing<String>>, String> {
    let account = keychain_account(secret_id, field);
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &account)
        .map_err(|e| keychain_error_message("entry", secret_id, field, e))?;
    match entry.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value.into())),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keychain_error_message("read", secret_id, field, e)),
    }
}

fn keychain_set(secret_id: &str, field: &str, value: &str) -> Result<(), String> {
    let account = keychain_account(secret_id, field);
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &account)
        .map_err(|e| keychain_error_message("entry", secret_id, field, e))?;
    if value.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keychain_error_message("delete", secret_id, field, e)),
        }
    } else {
        entry
            .set_password(value)
            .map_err(|e| keychain_error_message("store", secret_id, field, e))
    }
}

fn load_legacy_keychain_field(
    secret_id: &str,
    field: &str,
    label: &str,
    target: &mut Zeroizing<String>,
    errors: &mut Vec<String>,
) {
    if !target.trim().is_empty() {
        return;
    }

    match keychain_get(secret_id, field) {
        Ok(Some(secret)) => *target = secret,
        Ok(None) => {}
        Err(e) => errors.push(format!("{label} read failed: {e}")),
    }
}

pub fn load_keychain_secret_payload() -> Result<Option<SecretPayload>, String> {
    if in_memory_config_mode() {
        return Ok(None);
    }

    #[cfg(target_os = "windows")]
    if let Some(payload) = load_sharded_keychain_secret_payload()? {
        return Ok(Some(payload));
    }

    let Some(json) = keychain_get(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD)? else {
        return Ok(None);
    };

    let payload: SecretPayload = serde_json::from_str(json.as_str())
        .map_err(|e| redacted_secret_payload_parse_error("keychain payload parse failed", e))?;
    if payload.schema != SECRET_PAYLOAD_SCHEMA {
        return Err(format!(
            "keychain payload schema is unsupported; expected '{SECRET_PAYLOAD_SCHEMA}'"
        ));
    }
    Ok(Some(payload))
}

pub fn store_secret_payload(payload: &SecretPayload) -> Result<(), String> {
    if in_memory_config_mode() {
        return Ok(());
    }

    if payload.is_empty() {
        return clear_keychain_secret_payload();
    }

    let mut payload = payload.clone();
    payload.schema = SECRET_PAYLOAD_SCHEMA.to_string();

    #[cfg(target_os = "windows")]
    return store_sharded_keychain_secret_payload(&payload);

    #[cfg(not(target_os = "windows"))]
    let json = Zeroizing::new(
        serde_json::to_string(&payload)
            .map_err(|e| format!("keychain payload encode failed: {e}"))?,
    );

    #[cfg(not(target_os = "windows"))]
    return keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD, json.as_str());
}

pub(crate) fn update_keychain_secret_payload(
    update: KeychainSecretUpdate<'_>,
) -> Result<Option<String>, String> {
    if in_memory_config_mode() {
        return Ok(None);
    }

    update_keychain_secret_payload_with(
        update,
        KeychainUpdateHooks {
            load_payload: load_keychain_secret_payload,
            store_payload: store_secret_payload,
            clear_payload: clear_keychain_secret_payload,
            cleanup_legacy: || cleanup_legacy_keychain_update(update),
        },
    )
}

struct KeychainUpdateHooks<LoadPayload, StorePayload, ClearPayload, CleanupLegacy> {
    load_payload: LoadPayload,
    store_payload: StorePayload,
    clear_payload: ClearPayload,
    cleanup_legacy: CleanupLegacy,
}

fn update_keychain_secret_payload_with<LoadPayload, StorePayload, ClearPayload, CleanupLegacy>(
    update: KeychainSecretUpdate<'_>,
    mut hooks: KeychainUpdateHooks<LoadPayload, StorePayload, ClearPayload, CleanupLegacy>,
) -> Result<Option<String>, String>
where
    LoadPayload: FnMut() -> Result<Option<SecretPayload>, String>,
    StorePayload: FnMut(&SecretPayload) -> Result<(), String>,
    ClearPayload: FnMut() -> Result<(), String>,
    CleanupLegacy: FnMut() -> Result<(), String>,
{
    validate_keychain_secret_update(update)?;

    // Reading first is a hard safety boundary. If the current bundle cannot
    // be read, no partial runtime snapshot is allowed to replace it.
    let previous_payload = (hooks.load_payload)()
        .map_err(|error| format!("credential bundle snapshot failed: {error}"))?;
    let mut updated_payload = previous_payload.clone().unwrap_or_default();
    apply_keychain_secret_update(&mut updated_payload, update);

    (hooks.store_payload)(&updated_payload)?;

    if let Err(cleanup_error) = (hooks.cleanup_legacy)() {
        let cleanup_error = redact_sensitive_response_text(&cleanup_error);
        if keychain_update_explicitly_clears(update) {
            let rollback_result = match previous_payload {
                Some(payload) => (hooks.store_payload)(&payload),
                None => (hooks.clear_payload)(),
            };
            let mut error = format!(
                "required legacy credential cleanup failed; credential update was rolled back: {cleanup_error}"
            );
            if let Err(rollback_error) = rollback_result {
                error.push_str("; credential bundle rollback failed: ");
                error.push_str(&redact_sensitive_response_text(&rollback_error));
            }
            return Err(error);
        }

        return Ok(Some(cleanup_error));
    }

    Ok(None)
}

fn validate_keychain_secret_update(update: KeychainSecretUpdate<'_>) -> Result<(), String> {
    match update {
        KeychainSecretUpdate::Profile(profile) if profile.secret_id.trim().is_empty() => {
            Err("account credential update is missing its storage identifier".to_string())
        }
        KeychainSecretUpdate::RemoveProfile(secret_id) if secret_id.trim().is_empty() => {
            Err("account credential removal is missing its storage identifier".to_string())
        }
        _ => Ok(()),
    }
}

fn apply_keychain_secret_update(payload: &mut SecretPayload, update: KeychainSecretUpdate<'_>) {
    match update {
        KeychainSecretUpdate::Profile(profile) => {
            payload.upsert_profile_agent_key_for_wallet(
                &profile.secret_id,
                Some(&profile.wallet_address),
                &profile.agent_key,
            );
        }
        KeychainSecretUpdate::RemoveProfile(secret_id) => {
            payload.remove_profile(secret_id);
        }
        KeychainSecretUpdate::Hydromancer(value) => {
            payload.set_global_hydromancer_api_key(value);
        }
        KeychainSecretUpdate::Hyperdash(value) => {
            payload.set_global_hyperdash_api_key(value);
        }
        KeychainSecretUpdate::XOAuth {
            access_token,
            client_id,
            refresh_token,
        } => {
            payload.set_global_x_access_token(access_token);
            payload.set_global_x_oauth_client_id(client_id);
            payload.set_global_x_refresh_token(refresh_token);
        }
        KeychainSecretUpdate::SchwabOAuth {
            client_id,
            client_secret,
            access_token,
            refresh_token,
        } => {
            payload.set_global_schwab_client_id(client_id);
            payload.set_global_schwab_client_secret(client_secret);
            payload.set_global_schwab_access_token(access_token);
            payload.set_global_schwab_refresh_token(refresh_token);
        }
        KeychainSecretUpdate::OpenRouter(value) => {
            payload.set_global_openrouter_api_key(value);
        }
    }
}

fn keychain_update_explicitly_clears(update: KeychainSecretUpdate<'_>) -> bool {
    match update {
        KeychainSecretUpdate::Profile(profile) => profile.agent_key.trim().is_empty(),
        KeychainSecretUpdate::RemoveProfile(_) => true,
        KeychainSecretUpdate::Hydromancer(value)
        | KeychainSecretUpdate::Hyperdash(value)
        | KeychainSecretUpdate::OpenRouter(value) => value.trim().is_empty(),
        KeychainSecretUpdate::XOAuth {
            access_token,
            client_id,
            refresh_token,
        } => {
            access_token.trim().is_empty()
                && client_id.trim().is_empty()
                && refresh_token.trim().is_empty()
        }
        KeychainSecretUpdate::SchwabOAuth {
            client_id,
            client_secret,
            access_token,
            refresh_token,
        } => {
            client_id.trim().is_empty()
                && client_secret.trim().is_empty()
                && access_token.trim().is_empty()
                && refresh_token.trim().is_empty()
        }
    }
}

fn cleanup_legacy_keychain_update(update: KeychainSecretUpdate<'_>) -> Result<(), String> {
    match update {
        KeychainSecretUpdate::Profile(profile) if profile.agent_key.trim().is_empty() => {
            clear_legacy_profile_secret_entries_by_id(&profile.secret_id)
        }
        KeychainSecretUpdate::Profile(profile) => keychain_set(&profile.secret_id, "agent_key", ""),
        KeychainSecretUpdate::RemoveProfile(secret_id) => {
            clear_legacy_profile_secret_entries_by_id(secret_id)
        }
        KeychainSecretUpdate::Hydromancer(_) => {
            clear_legacy_global_secret_field("hydromancer_api_key")
        }
        KeychainSecretUpdate::Hyperdash(_) => clear_legacy_global_secret_field("hyperdash_api_key"),
        KeychainSecretUpdate::XOAuth { .. }
        | KeychainSecretUpdate::SchwabOAuth { .. }
        | KeychainSecretUpdate::OpenRouter(_) => Ok(()),
    }
}

#[cfg(target_os = "windows")]
fn load_sharded_keychain_secret_payload() -> Result<Option<SecretPayload>, String> {
    let Some(manifest_json) = keychain_get(GLOBAL_SECRET_ID, KEYCHAIN_SHARD_MANIFEST_FIELD)? else {
        return Ok(None);
    };
    let manifest = parse_keychain_shard_manifest(manifest_json.as_str())?;
    let mut json = Zeroizing::new(String::new());
    for index in 0..manifest.chunks {
        let field = keychain_shard_field(&manifest.generation, index);
        let Some(chunk) = keychain_get(GLOBAL_SECRET_ID, &field)? else {
            return Err("keychain credential bundle is incomplete".to_string());
        };
        json.push_str(chunk.as_str());
    }

    let payload: SecretPayload = serde_json::from_str(json.as_str())
        .map_err(|e| redacted_secret_payload_parse_error("keychain payload parse failed", e))?;
    if payload.schema != SECRET_PAYLOAD_SCHEMA {
        return Err(format!(
            "keychain payload schema is unsupported; expected '{SECRET_PAYLOAD_SCHEMA}'"
        ));
    }
    Ok(Some(payload))
}

#[cfg(target_os = "windows")]
fn store_sharded_keychain_secret_payload(payload: &SecretPayload) -> Result<(), String> {
    let json = Zeroizing::new(
        serde_json::to_string(payload)
            .map_err(|e| format!("keychain payload encode failed: {e}"))?,
    );
    let chunks = split_keychain_payload(json.as_str());
    if chunks.is_empty() || chunks.len() > KEYCHAIN_MAX_SHARDS {
        return Err("keychain credential bundle is too large".to_string());
    }

    let previous_manifest = keychain_get(GLOBAL_SECRET_ID, KEYCHAIN_SHARD_MANIFEST_FIELD)?
        .map(|json| parse_keychain_shard_manifest(json.as_str()))
        .transpose()?;
    let generation = new_keychain_shard_generation();
    let mut written = 0;
    for (index, chunk) in chunks.iter().enumerate() {
        let field = keychain_shard_field(&generation, index);
        if let Err(error) = keychain_set(GLOBAL_SECRET_ID, &field, chunk) {
            clear_keychain_shard_generation_best_effort(&generation, written);
            return Err(error);
        }
        written += 1;
    }

    let manifest = KeychainShardManifest {
        schema: KEYCHAIN_SHARD_SCHEMA.to_string(),
        generation: generation.clone(),
        chunks: chunks.len(),
    };
    let manifest_json = Zeroizing::new(
        serde_json::to_string(&manifest)
            .map_err(|e| format!("keychain shard manifest encode failed: {e}"))?,
    );
    if let Err(error) = keychain_set(
        GLOBAL_SECRET_ID,
        KEYCHAIN_SHARD_MANIFEST_FIELD,
        manifest_json.as_str(),
    ) {
        clear_keychain_shard_generation_best_effort(&generation, written);
        return Err(error);
    }

    let mut cleanup_errors = Vec::new();
    if let Some(previous) = previous_manifest
        && previous.generation != generation
        && let Err(error) = clear_keychain_shard_generation(&previous)
    {
        cleanup_errors.push(error);
    }
    if let Err(error) = keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD, "") {
        cleanup_errors.push(error);
    }

    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "keychain credential bundle saved but old credential cleanup failed: {}",
            cleanup_errors.join("; ")
        ))
    }
}

#[cfg(target_os = "windows")]
fn parse_keychain_shard_manifest(json: &str) -> Result<KeychainShardManifest, String> {
    let manifest: KeychainShardManifest =
        serde_json::from_str(json).map_err(|_| "keychain shard manifest is invalid".to_string())?;
    let safe_generation = !manifest.generation.is_empty()
        && manifest
            .generation
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if manifest.schema != KEYCHAIN_SHARD_SCHEMA
        || !safe_generation
        || manifest.chunks == 0
        || manifest.chunks > KEYCHAIN_MAX_SHARDS
    {
        return Err("keychain shard manifest is invalid".to_string());
    }
    Ok(manifest)
}

#[cfg(target_os = "windows")]
fn split_keychain_payload(value: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut chunk = String::new();
    let mut utf16_len = 0;
    for character in value.chars() {
        let character_len = character.len_utf16();
        if utf16_len + character_len > KEYCHAIN_SHARD_UTF16_LIMIT && !chunk.is_empty() {
            chunks.push(std::mem::take(&mut chunk));
            utf16_len = 0;
        }
        chunk.push(character);
        utf16_len += character_len;
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}

#[cfg(target_os = "windows")]
fn new_keychain_shard_generation() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = KEYCHAIN_SHARD_GENERATION.fetch_add(1, Ordering::Relaxed);
    format!("{}-{nanos}-{counter}", std::process::id())
}

#[cfg(target_os = "windows")]
fn keychain_shard_field(generation: &str, index: usize) -> String {
    format!("{KEYCHAIN_SHARD_FIELD_PREFIX}:{generation}:{index}")
}

#[cfg(target_os = "windows")]
fn clear_keychain_shard_generation(manifest: &KeychainShardManifest) -> Result<(), String> {
    let mut errors = Vec::new();
    for index in 0..manifest.chunks {
        let field = keychain_shard_field(&manifest.generation, index);
        if let Err(error) = keychain_set(GLOBAL_SECRET_ID, &field, "") {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(target_os = "windows")]
fn clear_keychain_shard_generation_best_effort(generation: &str, chunks: usize) {
    for index in 0..chunks {
        let field = keychain_shard_field(generation, index);
        let _ = keychain_set(GLOBAL_SECRET_ID, &field, "");
    }
}

#[allow(clippy::too_many_arguments)]
pub fn store_keychain_secrets_with_profile_removals_with_integrations(
    profiles: &[AccountProfile],
    hydromancer_api_key: &str,
    hyperdash_api_key: &str,
    x_access_token: &str,
    x_oauth_client_id: &str,
    x_refresh_token: &str,
    schwab_client_id: &str,
    schwab_client_secret: &str,
    schwab_access_token: &str,
    schwab_refresh_token: &str,
    openrouter_api_key: &str,
    removed_profile_secret_ids: &[String],
) -> Result<Option<String>, String> {
    if in_memory_config_mode() {
        return Ok(None);
    }

    store_keychain_secrets_with_profile_removals_with(
        profiles,
        hydromancer_api_key,
        hyperdash_api_key,
        x_access_token,
        x_oauth_client_id,
        x_refresh_token,
        schwab_client_id,
        schwab_client_secret,
        schwab_access_token,
        schwab_refresh_token,
        openrouter_api_key,
        removed_profile_secret_ids,
        KeychainProfileRemovalStoreHooks {
            load_payload: load_keychain_secret_payload,
            store_payload: store_secret_payload,
            clear_payload: clear_keychain_secret_payload,
            clear_legacy_after_bundle_store: clear_legacy_keychain_entries_after_bundle_store,
            clear_removed_profile: clear_legacy_profile_secret_entries_by_id,
        },
    )
}

struct KeychainProfileRemovalStoreHooks<
    LoadPayload,
    StorePayload,
    ClearPayload,
    ClearBundleLegacy,
    ClearRemovedProfile,
> {
    load_payload: LoadPayload,
    store_payload: StorePayload,
    clear_payload: ClearPayload,
    clear_legacy_after_bundle_store: ClearBundleLegacy,
    clear_removed_profile: ClearRemovedProfile,
}

#[allow(clippy::too_many_arguments)]
fn store_keychain_secrets_with_profile_removals_with<
    LoadPayload,
    StorePayload,
    ClearPayload,
    ClearBundleLegacy,
    ClearRemovedProfile,
>(
    profiles: &[AccountProfile],
    hydromancer_api_key: &str,
    hyperdash_api_key: &str,
    x_access_token: &str,
    x_oauth_client_id: &str,
    x_refresh_token: &str,
    schwab_client_id: &str,
    schwab_client_secret: &str,
    schwab_access_token: &str,
    schwab_refresh_token: &str,
    openrouter_api_key: &str,
    removed_profile_secret_ids: &[String],
    mut hooks: KeychainProfileRemovalStoreHooks<
        LoadPayload,
        StorePayload,
        ClearPayload,
        ClearBundleLegacy,
        ClearRemovedProfile,
    >,
) -> Result<Option<String>, String>
where
    LoadPayload: FnMut() -> Result<Option<SecretPayload>, String>,
    StorePayload: FnMut(&SecretPayload) -> Result<(), String>,
    ClearPayload: FnMut() -> Result<(), String>,
    ClearBundleLegacy: FnMut(&SecretPayload) -> Result<(), String>,
    ClearRemovedProfile: FnMut(&str) -> Result<(), String>,
{
    let payload = SecretPayload::from_credentials_with_integrations(
        profiles,
        hydromancer_api_key,
        hyperdash_api_key,
        x_access_token,
        x_oauth_client_id,
        x_refresh_token,
        schwab_client_id,
        schwab_client_secret,
        schwab_access_token,
        schwab_refresh_token,
        openrouter_api_key,
    );
    let requires_removed_profile_cleanup = removed_profile_secret_ids
        .iter()
        .any(|secret_id| removed_profile_legacy_cleanup_required(secret_id, &payload));
    let previous_payload = if requires_removed_profile_cleanup {
        Some(
            (hooks.load_payload)()
                .map_err(|error| format!("credential bundle snapshot failed: {error}"))?,
        )
    } else {
        None
    };

    (hooks.store_payload)(&payload)?;
    let cleanup_warning = (hooks.clear_legacy_after_bundle_store)(&payload).err();

    let mut removal_errors = Vec::new();
    for secret_id in removed_profile_secret_ids {
        let secret_id = secret_id.trim();
        if !removed_profile_legacy_cleanup_required(secret_id, &payload) {
            continue;
        }
        if let Err(error) = (hooks.clear_removed_profile)(secret_id) {
            removal_errors.push(profile_cleanup_error(secret_id, &error));
        }
    }

    if !removal_errors.is_empty() {
        let mut error = combined_keychain_cleanup_warning(cleanup_warning, removal_errors)
            .unwrap_or_else(|| "required profile credential cleanup failed".to_string());
        if let Some(previous_payload) = previous_payload {
            let rollback_result = match previous_payload {
                Some(payload) => (hooks.store_payload)(&payload),
                None => (hooks.clear_payload)(),
            };
            if let Err(rollback_error) = rollback_result {
                error.push_str("; credential bundle rollback failed: ");
                error.push_str(&rollback_error);
            }
        }
        return Err(error);
    }

    Ok(cleanup_warning)
}

fn removed_profile_legacy_cleanup_required(secret_id: &str, payload: &SecretPayload) -> bool {
    let secret_id = secret_id.trim();
    !secret_id.is_empty()
        && !payload
            .profiles
            .iter()
            .any(|profile| profile.secret_id == secret_id)
}

pub fn load_profile_secrets(profile: &mut AccountProfile) -> Result<(), String> {
    if profile.secret_id.is_empty() {
        profile.secret_id = new_secret_id();
    }
    if in_memory_config_mode() {
        return Ok(());
    }

    let mut errors = Vec::new();
    load_legacy_keychain_field(
        &profile.secret_id,
        "agent_key",
        "agent key",
        &mut profile.agent_key,
        &mut errors,
    );
    load_legacy_keychain_field(
        &profile.secret_id,
        "hydromancer_api_key",
        "Hydromancer key",
        &mut profile.hydromancer_api_key,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(profile_read_error(&profile.secret_id, &errors.join("; ")))
    }
}

pub fn load_global_secrets(
    hydromancer_api_key: &mut Zeroizing<String>,
    hyperdash_api_key: &mut Zeroizing<String>,
) -> Result<(), String> {
    if in_memory_config_mode() {
        return Ok(());
    }

    let mut errors = Vec::new();
    load_legacy_keychain_field(
        GLOBAL_SECRET_ID,
        "hydromancer_api_key",
        "Hydromancer key",
        hydromancer_api_key,
        &mut errors,
    );
    load_legacy_keychain_field(
        GLOBAL_SECRET_ID,
        "hyperdash_api_key",
        "HyperDash key",
        hyperdash_api_key,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn clear_profile_secrets(profile: &AccountProfile) -> Result<(), String> {
    if in_memory_config_mode() {
        return Ok(());
    }

    clear_profile_secrets_with(
        profile,
        load_keychain_secret_payload,
        store_secret_payload,
        clear_legacy_profile_secret_entries,
    )
}

pub(crate) fn clear_profile_secrets_by_id(secret_id: &str) -> Result<(), String> {
    let profile = AccountProfile {
        secret_id: secret_id.to_string(),
        name: String::new(),
        wallet_address: String::new(),
        agent_key: String::new().into(),
        hydromancer_api_key: String::new().into(),
    };
    clear_profile_secrets(&profile)
}

fn clear_profile_secrets_with(
    profile: &AccountProfile,
    mut load_payload: impl FnMut() -> Result<Option<SecretPayload>, String>,
    mut store_payload: impl FnMut(&SecretPayload) -> Result<(), String>,
    mut clear_legacy_profile: impl FnMut(&AccountProfile) -> Result<(), String>,
) -> Result<(), String> {
    let mut errors = Vec::new();

    let payload_without_profile = match load_payload() {
        Ok(Some(mut payload)) => payload
            .remove_profile(&profile.secret_id)
            .then_some(payload),
        Ok(None) => None,
        Err(e) => {
            return Err(format!("credential bundle read failed: {e}"));
        }
    };

    if let Err(e) = clear_legacy_profile(profile) {
        errors.push(profile_cleanup_error(&profile.secret_id, &e));
    }

    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    if let Some(payload) = payload_without_profile
        && let Err(e) = store_payload(&payload)
    {
        errors.push(format!("credential bundle update failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn profile_cleanup_error(secret_id: &str, error: &str) -> String {
    let error = redacted_cleanup_error(secret_id, error);
    if error.trim().is_empty() {
        "profile credential cleanup failed".to_string()
    } else {
        format!("profile credential cleanup failed: {error}")
    }
}

fn profile_read_error(secret_id: &str, error: &str) -> String {
    let error = redacted_cleanup_error(secret_id, error);
    if error.trim().is_empty() {
        "profile credential read failed".to_string()
    } else {
        error
    }
}

fn redacted_cleanup_error(secret_id: &str, error: &str) -> String {
    let secret_id = secret_id.trim();
    let redacted = if secret_id.is_empty() {
        error.to_string()
    } else {
        error.replace(secret_id, "<redacted-profile>")
    };
    redact_sensitive_response_text(&redacted)
}

fn clear_legacy_profile_secret_entries(profile: &AccountProfile) -> Result<(), String> {
    clear_legacy_profile_secret_entries_by_id(&profile.secret_id)
}

fn clear_legacy_profile_secret_entries_by_id(secret_id: &str) -> Result<(), String> {
    clear_legacy_profile_secret_entries_by_id_with(secret_id, |secret_id, field| {
        keychain_set(secret_id, field, "")
    })
}

fn clear_legacy_profile_secret_entries_by_id_with(
    secret_id: &str,
    mut clear_field: impl FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(e) = clear_field(secret_id, "agent_key") {
        errors.push(format!("agent key delete failed: {e}"));
    }
    if let Err(e) = clear_field(secret_id, "hydromancer_api_key") {
        errors.push(format!("Hydromancer key delete failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn clear_legacy_global_secret_field(field: &str) -> Result<(), String> {
    keychain_set(GLOBAL_SECRET_ID, field, "")
}

fn clear_legacy_global_secret_entries() -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(e) = clear_legacy_global_secret_field("hydromancer_api_key") {
        errors.push(format!("Hydromancer key delete failed: {e}"));
    }
    if let Err(e) = clear_legacy_global_secret_field("hyperdash_api_key") {
        errors.push(format!("HyperDash key delete failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn clear_secret_payload_entry() -> Result<(), String> {
    keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD, "")
}

pub fn clear_keychain_secret_payload() -> Result<(), String> {
    if in_memory_config_mode() {
        return Ok(());
    }

    let mut errors = Vec::new();

    #[cfg(target_os = "windows")]
    {
        match keychain_get(GLOBAL_SECRET_ID, KEYCHAIN_SHARD_MANIFEST_FIELD) {
            Ok(Some(json)) => match parse_keychain_shard_manifest(json.as_str()) {
                Ok(manifest) => {
                    if let Err(error) = clear_keychain_shard_generation(&manifest) {
                        errors.push(error);
                    }
                }
                Err(error) => errors.push(error),
            },
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
        if let Err(error) = keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_SHARD_MANIFEST_FIELD, "") {
            errors.push(error);
        }
    }

    if let Err(error) = clear_secret_payload_entry() {
        errors.push(error);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn combined_keychain_cleanup_warning(
    cleanup_warning: Option<String>,
    removal_errors: Vec<String>,
) -> Option<String> {
    let mut warnings = Vec::new();
    if let Some(cleanup_warning) = cleanup_warning
        && !cleanup_warning.trim().is_empty()
    {
        warnings.push(cleanup_warning);
    }
    warnings.extend(
        removal_errors
            .into_iter()
            .filter(|error| !error.trim().is_empty()),
    );

    (!warnings.is_empty()).then(|| warnings.join("; "))
}

pub fn clear_legacy_keychain_entries_for_payload(payload: &SecretPayload) -> Result<(), String> {
    if in_memory_config_mode() {
        return Ok(());
    }

    clear_legacy_keychain_entries_for_payload_with(
        payload,
        clear_legacy_profile_secret_entries_by_id,
        clear_legacy_global_secret_field,
    )
}

fn clear_legacy_keychain_entries_for_payload_with(
    payload: &SecretPayload,
    mut clear_profile: impl FnMut(&str) -> Result<(), String>,
    mut clear_global: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    for profile in &payload.profiles {
        if profile.secret_id.trim().is_empty() {
            continue;
        }
        if let Err(e) = clear_profile(&profile.secret_id) {
            errors.push(profile_cleanup_error(&profile.secret_id, &e));
        }
    }

    if let Err(e) = clear_global("hydromancer_api_key") {
        errors.push(format!("shared credential cleanup failed: {e}"));
    }
    if let Err(e) = clear_global("hyperdash_api_key") {
        errors.push(format!("shared credential cleanup failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn clear_legacy_keychain_entries_after_bundle_store(payload: &SecretPayload) -> Result<(), String> {
    let mut errors = Vec::new();
    for profile in &payload.profiles {
        if profile.secret_id.trim().is_empty() {
            continue;
        }
        if let Err(e) = clear_legacy_profile_secret_entries_by_id(&profile.secret_id) {
            errors.push(profile_cleanup_error(&profile.secret_id, &e));
        }
    }
    if let Err(e) = clear_legacy_global_secret_entries() {
        errors.push(format!("shared credential cleanup failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn clear_all_keychain_secrets(profiles: &[AccountProfile]) -> Result<(), String> {
    if in_memory_config_mode() {
        return Ok(());
    }

    let mut errors = Vec::new();
    for profile in profiles {
        if profile.secret_id.trim().is_empty() {
            continue;
        }
        if let Err(e) = clear_legacy_profile_secret_entries(profile) {
            errors.push(profile_cleanup_error(&profile.secret_id, &e));
        }
    }
    if let Err(e) = clear_keychain_secret_payload() {
        errors.push(format!("credential bundle delete failed: {e}"));
    }
    if let Err(e) = clear_legacy_global_secret_entries() {
        errors.push(format!("shared credential cleanup failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn test_profile(secret_id: &str) -> AccountProfile {
        AccountProfile {
            secret_id: secret_id.to_string(),
            name: secret_id.to_string(),
            wallet_address: "0xabc0000000000000000000000000000000000000".to_string(),
            agent_key: "agent-key".to_string().into(),
            hydromancer_api_key: String::new().into(),
        }
    }

    type TestKeychainUpdateHooks<'a> = KeychainUpdateHooks<
        Box<dyn FnMut() -> Result<Option<SecretPayload>, String> + 'a>,
        Box<dyn FnMut(&SecretPayload) -> Result<(), String> + 'a>,
        Box<dyn FnMut() -> Result<(), String> + 'a>,
        Box<dyn FnMut() -> Result<(), String> + 'a>,
    >;

    fn update_hooks<'a>(
        payload: &'a RefCell<Option<SecretPayload>>,
        cleanup_calls: &'a Cell<usize>,
    ) -> TestKeychainUpdateHooks<'a> {
        KeychainUpdateHooks {
            load_payload: Box::new(|| Ok(payload.borrow().clone())),
            store_payload: Box::new(|updated: &SecretPayload| {
                payload.borrow_mut().replace(updated.clone());
                Ok(())
            }),
            clear_payload: Box::new(|| {
                payload.borrow_mut().take();
                Ok(())
            }),
            cleanup_legacy: Box::new(|| {
                cleanup_calls.set(cleanup_calls.get() + 1);
                Ok(())
            }),
        }
    }

    #[test]
    fn scoped_updates_from_every_entry_flow_survive_as_one_complete_bundle() {
        let payload = RefCell::new(None);
        let cleanup_calls = Cell::new(0);
        let first_profile = test_profile("profile-a");
        let mut second_profile = test_profile("profile-b");
        second_profile.agent_key = "second-agent-key".to_string().into();

        for update in [
            KeychainSecretUpdate::Profile(&first_profile),
            KeychainSecretUpdate::Profile(&second_profile),
            KeychainSecretUpdate::Hydromancer("hydromancer-key"),
            KeychainSecretUpdate::Hyperdash("hyperdash-key"),
            KeychainSecretUpdate::XOAuth {
                access_token: "x-access-token",
                client_id: "x-client-id",
                refresh_token: "x-refresh-token",
            },
            KeychainSecretUpdate::SchwabOAuth {
                client_id: "schwab-client-id",
                client_secret: "schwab-client-secret",
                access_token: "schwab-access-token",
                refresh_token: "schwab-refresh-token",
            },
            KeychainSecretUpdate::OpenRouter("openrouter-key"),
        ] {
            update_keychain_secret_payload_with(update, update_hooks(&payload, &cleanup_calls))
                .expect("scoped update should succeed");
        }

        let payload = payload.into_inner().expect("stored credential bundle");
        assert_eq!(payload.profile_agent_key("profile-a"), Some("agent-key"));
        assert_eq!(
            payload.profile_agent_key("profile-b"),
            Some("second-agent-key")
        );
        assert_eq!(payload.global_hydromancer_api_key(), "hydromancer-key");
        assert_eq!(payload.global_hyperdash_api_key(), "hyperdash-key");
        assert_eq!(payload.global_x_access_token(), "x-access-token");
        assert_eq!(payload.global_x_oauth_client_id(), "x-client-id");
        assert_eq!(payload.global_x_refresh_token(), "x-refresh-token");
        assert_eq!(payload.global_schwab_client_id(), "schwab-client-id");
        assert_eq!(
            payload.global_schwab_client_secret(),
            "schwab-client-secret"
        );
        assert_eq!(payload.global_schwab_access_token(), "schwab-access-token");
        assert_eq!(
            payload.global_schwab_refresh_token(),
            "schwab-refresh-token"
        );
        assert_eq!(payload.global_openrouter_api_key(), "openrouter-key");
        assert_eq!(cleanup_calls.get(), 7);
    }

    #[test]
    fn scoped_clear_removes_only_the_explicit_credential() {
        let first_profile = test_profile("profile-a");
        let second_profile = test_profile("profile-b");
        let payload = RefCell::new(Some(SecretPayload::from_credentials_with_integrations(
            &[first_profile, second_profile],
            "hydromancer-key",
            "hyperdash-key",
            "x-access-token",
            "x-client-id",
            "x-refresh-token",
            "schwab-client-id",
            "schwab-client-secret",
            "schwab-access-token",
            "schwab-refresh-token",
            "openrouter-key",
        )));
        let cleanup_calls = Cell::new(0);

        update_keychain_secret_payload_with(
            KeychainSecretUpdate::OpenRouter(""),
            update_hooks(&payload, &cleanup_calls),
        )
        .expect("explicit clear should succeed");

        let payload = payload.into_inner().expect("remaining credential bundle");
        assert_eq!(payload.profile_agent_key("profile-a"), Some("agent-key"));
        assert_eq!(payload.profile_agent_key("profile-b"), Some("agent-key"));
        assert_eq!(payload.global_hydromancer_api_key(), "hydromancer-key");
        assert_eq!(payload.global_hyperdash_api_key(), "hyperdash-key");
        assert_eq!(payload.global_x_access_token(), "x-access-token");
        assert_eq!(payload.global_schwab_client_id(), "schwab-client-id");
        assert!(payload.global_openrouter_api_key().is_empty());
    }

    #[test]
    fn scoped_update_never_writes_when_existing_bundle_cannot_be_read() {
        let store_called = Cell::new(false);
        let clear_called = Cell::new(false);
        let cleanup_called = Cell::new(false);

        let error = update_keychain_secret_payload_with(
            KeychainSecretUpdate::Hydromancer("new-key"),
            KeychainUpdateHooks {
                load_payload: || Err("keychain unavailable".to_string()),
                store_payload: |_payload: &SecretPayload| {
                    store_called.set(true);
                    Ok(())
                },
                clear_payload: || {
                    clear_called.set(true);
                    Ok(())
                },
                cleanup_legacy: || {
                    cleanup_called.set(true);
                    Ok(())
                },
            },
        )
        .expect_err("unreadable bundle must block the update");

        assert!(error.contains("credential bundle snapshot failed"));
        assert!(!store_called.get());
        assert!(!clear_called.get());
        assert!(!cleanup_called.get());
    }

    #[test]
    fn explicit_clear_rolls_bundle_back_when_required_legacy_cleanup_fails() {
        let previous = SecretPayload::from_credentials(&[], "old-hydromancer", "hyperdash-key");
        let stored_payloads = RefCell::new(Vec::new());

        let error = update_keychain_secret_payload_with(
            KeychainSecretUpdate::Hydromancer(""),
            KeychainUpdateHooks {
                load_payload: || Ok(Some(previous.clone())),
                store_payload: |payload: &SecretPayload| {
                    stored_payloads.borrow_mut().push(payload.clone());
                    Ok(())
                },
                clear_payload: || panic!("a previous bundle exists"),
                cleanup_legacy: || Err("legacy key delete failed".to_string()),
            },
        )
        .expect_err("failed deletion cleanup must fail and roll back");

        assert!(error.contains("credential update was rolled back"));
        let stored_payloads = stored_payloads.borrow();
        assert_eq!(stored_payloads.len(), 2);
        assert!(stored_payloads[0].global_hydromancer_api_key().is_empty());
        assert_eq!(
            stored_payloads[1].global_hydromancer_api_key(),
            "old-hydromancer"
        );
        assert_eq!(
            stored_payloads[1].global_hyperdash_api_key(),
            "hyperdash-key"
        );
    }

    #[test]
    fn profile_cleanup_error_redacts_secret_id() {
        let rendered = profile_cleanup_error(
            "profile-secret-id",
            "delete profile-secret-id:agent_key failed",
        );

        assert!(rendered.contains("profile credential cleanup failed"));
        assert!(rendered.contains("<redacted-profile>"));
        assert!(!rendered.contains("profile-secret-id"));
    }

    #[test]
    fn keychain_error_message_redacts_profile_identifier_and_account_name() {
        let rendered = keychain_error_message(
            "read",
            "profile-secret-id",
            "agent_key",
            "backend denied profile-secret-id:agent_key for profile-secret-id",
        );

        assert!(rendered.contains("keychain read failed"));
        assert!(rendered.contains("<keychain-entry>"));
        assert!(rendered.contains("<redacted-profile>"));
        assert!(!rendered.contains("profile-secret-id"));
        assert!(!rendered.contains("profile-secret-id:agent_key"));
    }

    #[test]
    fn keychain_error_message_redacts_global_account_without_hiding_generic_global_text() {
        let rendered = keychain_error_message(
            "delete",
            GLOBAL_SECRET_ID,
            KEYCHAIN_PAYLOAD_FIELD,
            "global keychain denied global:secrets_v1",
        );

        assert!(rendered.contains("keychain delete failed"));
        assert!(rendered.contains("global keychain"));
        assert!(rendered.contains("<keychain-entry>"));
        assert!(!rendered.contains("global:secrets_v1"));
    }

    #[test]
    fn keychain_error_message_redacts_secret_like_backend_payload() {
        let rendered = keychain_error_message(
            "read",
            "profile-secret-id",
            "agent_key",
            "backend denied profile-secret-id:agent_key token=backend-secret",
        );

        assert!(rendered.contains("keychain read failed"));
        assert!(rendered.contains("<keychain-entry>"));
        assert!(rendered.contains("token=<redacted>"));
        assert!(!rendered.contains("profile-secret-id"));
        assert!(!rendered.contains("backend-secret"));
    }

    #[test]
    fn profile_cleanup_error_redacts_secret_like_payload() {
        let rendered = profile_cleanup_error(
            "profile-secret-id",
            "delete profile-secret-id failed api_key=cleanup-secret",
        );

        assert!(rendered.contains("profile credential cleanup failed"));
        assert!(rendered.contains("<redacted-profile>"));
        assert!(rendered.contains("api_key=<redacted>"));
        assert!(!rendered.contains("profile-secret-id"));
        assert!(!rendered.contains("cleanup-secret"));
    }

    #[test]
    fn profile_legacy_cleanup_clears_agent_and_hydromancer_fields() {
        let cleared_fields = RefCell::new(Vec::new());

        clear_legacy_profile_secret_entries_by_id_with("profile-secret-id", |secret_id, field| {
            cleared_fields
                .borrow_mut()
                .push((secret_id.to_string(), field.to_string()));
            Ok(())
        })
        .expect("cleanup should succeed");

        assert_eq!(
            cleared_fields.borrow().as_slice(),
            [
                ("profile-secret-id".to_string(), "agent_key".to_string()),
                (
                    "profile-secret-id".to_string(),
                    "hydromancer_api_key".to_string()
                ),
            ]
        );
    }

    #[test]
    fn payload_legacy_cleanup_clears_profiles_and_all_global_fields() {
        let profile = test_profile("profile-secret-id");
        let payload = SecretPayload::from_credentials(std::slice::from_ref(&profile), "", "");
        let cleared_profiles = RefCell::new(Vec::new());
        let cleared_globals = RefCell::new(Vec::new());

        clear_legacy_keychain_entries_for_payload_with(
            &payload,
            |secret_id| {
                cleared_profiles.borrow_mut().push(secret_id.to_string());
                Ok(())
            },
            |field| {
                cleared_globals.borrow_mut().push(field.to_string());
                Ok(())
            },
        )
        .expect("cleanup should succeed");

        assert_eq!(
            cleared_profiles.borrow().as_slice(),
            ["profile-secret-id".to_string()]
        );
        assert_eq!(
            cleared_globals.borrow().as_slice(),
            [
                "hydromancer_api_key".to_string(),
                "hyperdash_api_key".to_string(),
            ]
        );
    }

    #[test]
    fn profile_removal_cleanup_errors_are_returned_as_warnings() {
        let warning = combined_keychain_cleanup_warning(
            Some("bundle legacy cleanup failed".to_string()),
            vec![
                "profile credential cleanup failed".to_string(),
                " ".to_string(),
            ],
        )
        .expect("cleanup warnings should be combined");

        assert_eq!(
            warning,
            "bundle legacy cleanup failed; profile credential cleanup failed"
        );
    }

    #[test]
    fn required_removed_profile_cleanup_succeeds_before_returning_success() {
        let kept_profile = test_profile("kept-profile");
        let mut removed_profile = test_profile("removed-profile");
        removed_profile.agent_key = String::new().into();
        let stored_payloads = RefCell::new(Vec::new());
        let cleared_profiles = RefCell::new(Vec::new());
        let rollback_clear_called = Cell::new(false);

        let result = store_keychain_secrets_with_profile_removals_with(
            &[kept_profile.clone(), removed_profile.clone()],
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            &[removed_profile.secret_id.clone()],
            KeychainProfileRemovalStoreHooks {
                load_payload: || Ok(Some(SecretPayload::from_credentials(&[], "", ""))),
                store_payload: |payload: &SecretPayload| {
                    stored_payloads.borrow_mut().push(payload.clone());
                    Ok(())
                },
                clear_payload: || {
                    rollback_clear_called.set(true);
                    Ok(())
                },
                clear_legacy_after_bundle_store: |_payload: &SecretPayload| Ok(()),
                clear_removed_profile: |secret_id: &str| {
                    cleared_profiles.borrow_mut().push(secret_id.to_string());
                    Ok(())
                },
            },
        )
        .expect("required cleanup should succeed");

        assert_eq!(result, None);
        assert_eq!(
            cleared_profiles.borrow().as_slice(),
            ["removed-profile".to_string()]
        );
        let stored_payloads = stored_payloads.borrow();
        assert_eq!(stored_payloads.len(), 1);
        assert_eq!(
            stored_payloads[0].profile_agent_key("kept-profile"),
            Some("agent-key")
        );
        assert_eq!(
            stored_payloads[0].profile_agent_key("removed-profile"),
            None
        );
        assert!(!rollback_clear_called.get());
    }

    #[test]
    fn required_removed_profile_cleanup_failure_rolls_back_bundle_and_errors() {
        let kept_profile = test_profile("kept-profile");
        let mut removed_profile = test_profile("removed-profile");
        removed_profile.agent_key = String::new().into();
        let previous_removed_profile = test_profile("removed-profile");
        let previous_payload = SecretPayload::from_credentials(
            &[kept_profile.clone(), previous_removed_profile],
            "",
            "",
        );
        let stored_payloads = RefCell::new(Vec::new());

        let result = store_keychain_secrets_with_profile_removals_with(
            &[kept_profile, removed_profile.clone()],
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            &[removed_profile.secret_id.clone()],
            KeychainProfileRemovalStoreHooks {
                load_payload: || Ok(Some(previous_payload.clone())),
                store_payload: |payload: &SecretPayload| {
                    stored_payloads.borrow_mut().push(payload.clone());
                    Ok(())
                },
                clear_payload: || panic!("previous bundle exists, so rollback should restore it"),
                clear_legacy_after_bundle_store: |_payload: &SecretPayload| Ok(()),
                clear_removed_profile: |_secret_id: &str| {
                    Err("delete removed-profile:agent_key failed".to_string())
                },
            },
        );

        let error = result.expect_err("required cleanup failure should fail the save");
        assert!(error.contains("profile credential cleanup failed"));
        assert!(error.contains("<redacted-profile>"));
        assert!(!error.contains("removed-profile"));
        let stored_payloads = stored_payloads.borrow();
        assert_eq!(stored_payloads.len(), 2);
        assert_eq!(
            stored_payloads[0].profile_agent_key("removed-profile"),
            None
        );
        assert_eq!(
            stored_payloads[1].profile_agent_key("removed-profile"),
            Some("agent-key")
        );
    }

    #[test]
    fn profile_clear_does_not_store_bundle_when_legacy_cleanup_fails() {
        let profile = test_profile("profile-secret-id");
        let payload = SecretPayload::from_credentials(std::slice::from_ref(&profile), "", "");
        let stored_payloads = RefCell::new(Vec::new());

        let result = clear_profile_secrets_with(
            &profile,
            || Ok(Some(payload.clone())),
            |payload| {
                stored_payloads.borrow_mut().push(payload.clone());
                Ok(())
            },
            |_profile| Err("delete profile-secret-id:agent_key failed".to_string()),
        );

        let error = result.expect_err("legacy cleanup failure should abort profile removal");
        assert!(error.contains("profile credential cleanup failed"));
        assert!(error.contains("<redacted-profile>"));
        assert!(!error.contains("profile-secret-id"));
        assert!(
            stored_payloads.borrow().is_empty(),
            "bundle must not be rewritten after legacy cleanup failure"
        );
    }

    #[test]
    fn profile_clear_does_not_clear_legacy_when_bundle_read_fails() {
        let profile = test_profile("profile-secret-id");
        let legacy_clear_called = Cell::new(false);
        let store_called = Cell::new(false);

        let result = clear_profile_secrets_with(
            &profile,
            || Err("bundle unavailable".to_string()),
            |_payload| {
                store_called.set(true);
                Ok(())
            },
            |_profile| {
                legacy_clear_called.set(true);
                Ok(())
            },
        );

        let error = result.expect_err("bundle read failure should abort profile removal");
        assert!(error.contains("credential bundle read failed"));
        assert!(!legacy_clear_called.get());
        assert!(!store_called.get());
    }

    #[test]
    fn profile_clear_stores_bundle_after_legacy_cleanup_succeeds() {
        let removed_profile = test_profile("removed-profile");
        let kept_profile = test_profile("kept-profile");
        let payload = SecretPayload::from_credentials(
            &[removed_profile.clone(), kept_profile.clone()],
            "",
            "",
        );
        let order = RefCell::new(Vec::new());
        let stored_payloads = RefCell::new(Vec::new());

        let result = clear_profile_secrets_with(
            &removed_profile,
            || Ok(Some(payload.clone())),
            |payload| {
                order.borrow_mut().push("store-bundle".to_string());
                stored_payloads.borrow_mut().push(payload.clone());
                Ok(())
            },
            |profile| {
                assert_eq!(profile.secret_id, "removed-profile");
                order.borrow_mut().push("clear-legacy".to_string());
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert_eq!(order.borrow().as_slice(), ["clear-legacy", "store-bundle"]);
        let stored_payloads = stored_payloads.borrow();
        assert_eq!(stored_payloads.len(), 1);
        assert_eq!(
            stored_payloads[0].profile_agent_key("removed-profile"),
            None
        );
        assert_eq!(
            stored_payloads[0].profile_agent_key("kept-profile"),
            Some("agent-key")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_keychain_payload_chunks_stay_within_utf16_limit_and_round_trip() {
        let value = format!(
            "{}{}{}",
            "a".repeat(KEYCHAIN_SHARD_UTF16_LIMIT - 1),
            '🚀',
            "b".repeat(KEYCHAIN_SHARD_UTF16_LIMIT + 7)
        );

        let chunks = split_keychain_payload(&value);

        assert!(chunks.len() >= 3);
        assert!(
            chunks
                .iter()
                .all(|chunk| { chunk.encode_utf16().count() <= KEYCHAIN_SHARD_UTF16_LIMIT })
        );
        assert_eq!(chunks.concat(), value);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_keychain_manifest_rejects_unsafe_or_unbounded_values() {
        let unsafe_generation = format!(
            r#"{{"schema":"{KEYCHAIN_SHARD_SCHEMA}","generation":"../secret","chunks":1}}"#
        );
        let excessive_chunks = format!(
            r#"{{"schema":"{KEYCHAIN_SHARD_SCHEMA}","generation":"safe-1","chunks":{}}}"#,
            KEYCHAIN_MAX_SHARDS + 1
        );

        assert!(parse_keychain_shard_manifest(&unsafe_generation).is_err());
        assert!(parse_keychain_shard_manifest(&excessive_chunks).is_err());
    }
}
