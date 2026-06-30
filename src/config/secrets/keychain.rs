use super::super::{AccountProfile, new_secret_id};
use super::model::{SECRET_PAYLOAD_SCHEMA, SecretPayload, redacted_secret_payload_parse_error};
use crate::config::in_memory_config_mode;
use crate::helpers::redact_sensitive_response_text;
use zeroize::Zeroizing;

const KEYCHAIN_SERVICE: &str = "kerosene";
const GLOBAL_SECRET_ID: &str = "global";
const KEYCHAIN_PAYLOAD_FIELD: &str = "secrets_v1";

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
        return keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD, "");
    }

    let mut payload = payload.clone();
    payload.schema = SECRET_PAYLOAD_SCHEMA.to_string();
    let json = Zeroizing::new(
        serde_json::to_string(&payload)
            .map_err(|e| format!("keychain payload encode failed: {e}"))?,
    );
    keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD, json.as_str())
}

pub fn store_keychain_secrets_with_x(
    profiles: &[AccountProfile],
    hydromancer_api_key: &str,
    hyperdash_api_key: &str,
    x_access_token: &str,
) -> Result<Option<String>, String> {
    store_keychain_secrets_with_profile_removals_with_x(
        profiles,
        hydromancer_api_key,
        hyperdash_api_key,
        x_access_token,
        &[],
    )
}

pub fn store_keychain_secrets_with_profile_removals_with_x(
    profiles: &[AccountProfile],
    hydromancer_api_key: &str,
    hyperdash_api_key: &str,
    x_access_token: &str,
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
    let payload = SecretPayload::from_credentials_with_x(
        profiles,
        hydromancer_api_key,
        hyperdash_api_key,
        x_access_token,
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

    clear_secret_payload_entry()
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
    if let Err(e) = clear_secret_payload_entry() {
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
}
