use super::super::{AccountProfile, new_secret_id};
use super::model::{SECRET_PAYLOAD_SCHEMA, SecretPayload};
use zeroize::Zeroizing;

const KEYCHAIN_SERVICE: &str = "kerosene";
const GLOBAL_SECRET_ID: &str = "global";
const KEYCHAIN_PAYLOAD_FIELD: &str = "secrets_v1";

fn keychain_account(secret_id: &str, field: &str) -> String {
    format!("{secret_id}:{field}")
}

fn keychain_get(secret_id: &str, field: &str) -> Result<Option<Zeroizing<String>>, String> {
    let account = keychain_account(secret_id, field);
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &account)
        .map_err(|e| format!("keychain entry failed: {e}"))?;
    match entry.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value.into())),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keychain read failed: {e}")),
    }
}

fn keychain_set(secret_id: &str, field: &str, value: &str) -> Result<(), String> {
    let account = keychain_account(secret_id, field);
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &account)
        .map_err(|e| format!("keychain entry failed: {e}"))?;
    if value.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keychain delete failed: {e}")),
        }
    } else {
        entry
            .set_password(value)
            .map_err(|e| format!("keychain store failed: {e}"))
    }
}

pub fn load_keychain_secret_payload() -> Result<Option<SecretPayload>, String> {
    let Some(json) = keychain_get(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD)? else {
        return Ok(None);
    };

    let payload: SecretPayload = serde_json::from_str(json.as_str())
        .map_err(|e| format!("keychain payload parse failed: {e}"))?;
    if payload.schema != SECRET_PAYLOAD_SCHEMA {
        return Err(format!(
            "keychain payload schema is '{}', expected '{}'",
            payload.schema, SECRET_PAYLOAD_SCHEMA
        ));
    }
    Ok(Some(payload))
}

pub fn store_secret_payload(payload: &SecretPayload) -> Result<(), String> {
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

pub fn store_keychain_secrets(
    profiles: &[AccountProfile],
    hydromancer_api_key: &str,
    hyperdash_api_key: &str,
    x_bearer_token: &str,
) -> Result<Option<String>, String> {
    let payload = SecretPayload::from_credentials(
        profiles,
        hydromancer_api_key,
        hyperdash_api_key,
        x_bearer_token,
    );
    store_secret_payload(&payload)?;
    match clear_legacy_keychain_entries_for_payload(&payload) {
        Ok(()) => Ok(None),
        Err(error) => Ok(Some(error)),
    }
}

pub fn load_profile_secrets(profile: &mut AccountProfile) -> Result<(), String> {
    if profile.secret_id.is_empty() {
        profile.secret_id = new_secret_id();
    }

    let legacy_agent_key = std::mem::take(&mut profile.agent_key);
    let mut errors = Vec::new();

    if !legacy_agent_key.trim().is_empty() {
        profile.agent_key = legacy_agent_key;
    } else {
        match keychain_get(&profile.secret_id, "agent_key") {
            Ok(Some(secret)) => profile.agent_key = secret,
            Ok(None) => {}
            Err(e) => errors.push(format!("agent key read failed: {e}")),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn clear_profile_secrets(profile: &AccountProfile) -> Result<(), String> {
    let mut errors = Vec::new();

    match load_keychain_secret_payload() {
        Ok(Some(mut payload)) => {
            if payload.remove_profile(&profile.secret_id)
                && let Err(e) = store_secret_payload(&payload)
            {
                errors.push(format!("credential bundle update failed: {e}"));
            }
        }
        Ok(None) => {}
        Err(e) => errors.push(format!("credential bundle read failed: {e}")),
    }

    if let Err(e) = clear_legacy_profile_secret_entries(profile) {
        errors.push(e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn clear_legacy_profile_secret_entries(profile: &AccountProfile) -> Result<(), String> {
    clear_legacy_profile_secret_entries_by_id(&profile.secret_id)
}

fn clear_legacy_profile_secret_entries_by_id(secret_id: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(e) = keychain_set(secret_id, "agent_key", "") {
        errors.push(format!("agent key delete failed: {e}"));
    }
    if let Err(e) = keychain_set(secret_id, "hydromancer_api_key", "") {
        errors.push(format!("Hydromancer key delete failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn clear_legacy_global_secret_entries() -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hydromancer_api_key", "") {
        errors.push(format!("Hydromancer key delete failed: {e}"));
    }
    if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hyperdash_api_key", "") {
        errors.push(format!("HyperDash key delete failed: {e}"));
    }
    if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "x_bearer_token", "") {
        errors.push(format!("X bearer token delete failed: {e}"));
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

pub fn clear_legacy_keychain_entries_for_payload(payload: &SecretPayload) -> Result<(), String> {
    let mut errors = Vec::new();
    for profile in &payload.profiles {
        if profile.secret_id.trim().is_empty() {
            continue;
        }
        if let Err(e) = clear_legacy_profile_secret_entries_by_id(&profile.secret_id) {
            errors.push(format!("{}: {e}", profile.secret_id));
        }
    }
    if let Err(e) = clear_legacy_global_secret_entries() {
        errors.push(format!("global: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn clear_all_keychain_secrets(profiles: &[AccountProfile]) -> Result<(), String> {
    let mut errors = Vec::new();
    for profile in profiles {
        if profile.secret_id.trim().is_empty() {
            continue;
        }
        if let Err(e) = clear_legacy_profile_secret_entries(profile) {
            errors.push(format!("{}: {e}", profile.name));
        }
    }
    if let Err(e) = clear_secret_payload_entry() {
        errors.push(format!("credential bundle delete failed: {e}"));
    }
    if let Err(e) = clear_legacy_global_secret_entries() {
        errors.push(format!("global: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
