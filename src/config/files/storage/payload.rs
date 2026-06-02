use crate::config::{KeroseneConfig, SecretPayload, new_secret_id};

use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Secret Payload Hydration
// ---------------------------------------------------------------------------

pub(super) fn merge_missing_plaintext_secrets_into_payload(
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
    if payload.global_x_bearer_token().trim().is_empty() && !config.x_bearer_token.trim().is_empty()
    {
        changed |= payload.set_global_x_bearer_token(&config.x_bearer_token);
    }

    changed
}

pub(super) fn apply_secret_payload(config: &mut KeroseneConfig, payload: &SecretPayload) {
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
    config.x_bearer_token.zeroize();
    config.x_bearer_token = payload.global_x_bearer_token().to_string().into();
}

#[cfg(test)]
mod tests;
