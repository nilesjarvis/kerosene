use super::super::{AccountProfile, new_secret_id};
use super::model::{SECRET_PAYLOAD_SCHEMA, SecretPayload};
use super::warnings::push_secret_warning;

const KEYCHAIN_SERVICE: &str = "kerosene";
const GLOBAL_SECRET_ID: &str = "global";
const KEYCHAIN_PAYLOAD_FIELD: &str = "secrets_v1";

fn keychain_account(secret_id: &str, field: &str) -> String {
    format!("{secret_id}:{field}")
}

fn keychain_get(secret_id: &str, field: &str) -> Result<Option<String>, String> {
    let account = keychain_account(secret_id, field);
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &account)
        .map_err(|e| format!("keychain entry failed: {e}"))?;
    match entry.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
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

    let payload: SecretPayload =
        serde_json::from_str(&json).map_err(|e| format!("keychain payload parse failed: {e}"))?;
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
    let json = serde_json::to_string(&payload)
        .map_err(|e| format!("keychain payload encode failed: {e}"))?;
    keychain_set(GLOBAL_SECRET_ID, KEYCHAIN_PAYLOAD_FIELD, &json)
}

pub fn store_keychain_secrets(
    profiles: &[AccountProfile],
    hydromancer_api_key: &str,
    hyperdash_api_key: &str,
) -> Result<(), String> {
    let payload = SecretPayload::from_credentials(profiles, hydromancer_api_key, hyperdash_api_key);
    store_secret_payload(&payload)
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
            Ok(Some(secret)) => profile.agent_key = secret.into(),
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

pub fn load_profile_hydromancer_secret(profile: &AccountProfile) -> Result<Option<String>, String> {
    if profile.secret_id.trim().is_empty() {
        return Ok(None);
    }
    keychain_get(&profile.secret_id, "hydromancer_api_key")
        .map_err(|e| format!("Hydromancer key read failed: {e}"))
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

    if let Err(e) = keychain_set(&profile.secret_id, "agent_key", "") {
        errors.push(format!("agent key delete failed: {e}"));
    }
    if let Err(e) = keychain_set(&profile.secret_id, "hydromancer_api_key", "") {
        errors.push(format!("Hydromancer key delete failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn load_global_hydromancer_secret(legacy_value: String) -> String {
    match keychain_get(GLOBAL_SECRET_ID, "hydromancer_api_key") {
        Ok(Some(secret)) => secret,
        Ok(None) => {
            if !legacy_value.trim().is_empty() {
                legacy_value
            } else {
                String::new()
            }
        }
        Err(e) => {
            push_secret_warning(format!("Hydromancer key read failed: {e}"));
            legacy_value
        }
    }
}

pub fn load_global_hyperdash_secret(legacy_value: String) -> String {
    if !legacy_value.trim().is_empty() {
        legacy_value
    } else {
        match keychain_get(GLOBAL_SECRET_ID, "hyperdash_api_key") {
            Ok(Some(secret)) => secret,
            Ok(None) => String::new(),
            Err(e) => {
                push_secret_warning(format!("HyperDash key read failed: {e}"));
                String::new()
            }
        }
    }
}

pub fn clear_global_secrets() -> Result<(), String> {
    let mut errors = Vec::new();

    match load_keychain_secret_payload() {
        Ok(Some(mut payload)) => {
            let mut changed = payload.set_global_hydromancer_api_key("");
            changed |= payload.set_global_hyperdash_api_key("");
            if changed && let Err(e) = store_secret_payload(&payload) {
                errors.push(format!("credential bundle update failed: {e}"));
            }
        }
        Ok(None) => {}
        Err(e) => errors.push(format!("credential bundle read failed: {e}")),
    }

    if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hydromancer_api_key", "") {
        errors.push(format!("Hydromancer key delete failed: {e}"));
    }
    if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hyperdash_api_key", "") {
        errors.push(format!("HyperDash key delete failed: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
