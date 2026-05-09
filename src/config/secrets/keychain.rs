use super::super::{AccountProfile, new_secret_id};
use super::warnings::push_secret_warning;

const KEYCHAIN_SERVICE: &str = "kerosene";
const GLOBAL_SECRET_ID: &str = "global";

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

pub fn load_profile_secrets(profile: &mut AccountProfile) -> Result<(), String> {
    if profile.secret_id.is_empty() {
        profile.secret_id = new_secret_id();
    }

    let legacy_agent_key = std::mem::take(&mut profile.agent_key);
    let legacy_hydromancer_key = std::mem::take(&mut profile.hydromancer_api_key);
    let mut errors = Vec::new();

    if !legacy_agent_key.trim().is_empty() {
        if let Err(e) = keychain_set(&profile.secret_id, "agent_key", &legacy_agent_key) {
            errors.push(format!("agent key migration failed: {e}"));
        }
        profile.agent_key = legacy_agent_key;
    } else {
        match keychain_get(&profile.secret_id, "agent_key") {
            Ok(Some(secret)) => profile.agent_key = secret.into(),
            Ok(None) => {}
            Err(e) => errors.push(format!("agent key read failed: {e}")),
        }
    }

    if !legacy_hydromancer_key.trim().is_empty() {
        if let Err(e) = keychain_set(
            &profile.secret_id,
            "hydromancer_api_key",
            &legacy_hydromancer_key,
        ) {
            errors.push(format!("Hydromancer key migration failed: {e}"));
        }
        profile.hydromancer_api_key = legacy_hydromancer_key;
    } else {
        match keychain_get(&profile.secret_id, "hydromancer_api_key") {
            Ok(Some(secret)) => profile.hydromancer_api_key = secret.into(),
            Ok(None) => {}
            Err(e) => errors.push(format!("Hydromancer key read failed: {e}")),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn store_profile_secrets(profile: &AccountProfile) -> Result<(), String> {
    keychain_set(&profile.secret_id, "agent_key", &profile.agent_key)
}

pub fn clear_profile_secrets(profile: &AccountProfile) -> Result<(), String> {
    let mut errors = Vec::new();

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

pub fn clear_profile_hydromancer_secret(profile: &AccountProfile) -> Result<(), String> {
    keychain_set(&profile.secret_id, "hydromancer_api_key", "")
}

pub fn load_global_hydromancer_secret(legacy_value: String) -> String {
    match keychain_get(GLOBAL_SECRET_ID, "hydromancer_api_key") {
        Ok(Some(secret)) => secret,
        Ok(None) => {
            if !legacy_value.trim().is_empty() {
                if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hydromancer_api_key", &legacy_value)
                {
                    push_secret_warning(format!("Hydromancer key migration failed: {e}"));
                }
                legacy_value
            } else {
                String::new()
            }
        }
        Err(e) => {
            push_secret_warning(format!("Hydromancer key read failed: {e}"));
            if !legacy_value.trim().is_empty() {
                if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hydromancer_api_key", &legacy_value)
                {
                    push_secret_warning(format!("Hydromancer key migration failed: {e}"));
                }
                legacy_value
            } else {
                String::new()
            }
        }
    }
}

pub fn store_global_hydromancer_secret(value: &str) -> Result<(), String> {
    keychain_set(GLOBAL_SECRET_ID, "hydromancer_api_key", value)
}

pub fn load_global_hyperdash_secret(legacy_value: String) -> String {
    if !legacy_value.trim().is_empty() {
        if let Err(e) = keychain_set(GLOBAL_SECRET_ID, "hyperdash_api_key", &legacy_value) {
            push_secret_warning(format!("HyperDash key migration failed: {e}"));
        }
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

pub fn store_global_hyperdash_secret(value: &str) -> Result<(), String> {
    keychain_set(GLOBAL_SECRET_ID, "hyperdash_api_key", value)
}

pub fn clear_global_secrets() -> Result<(), String> {
    let mut errors = Vec::new();

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
