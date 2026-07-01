use crate::config::{KeroseneConfig, SecretPayload, new_secret_id};

use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Secret Payload Hydration
// ---------------------------------------------------------------------------

fn push_wallet_binding_mismatch_warning() {
    crate::config::push_secret_warning(
        "Saved agent key for an account was not loaded because it is bound to a different wallet address. Re-enter and save credentials to trade from this account."
            .to_string(),
    );
}

pub(super) fn merge_missing_plaintext_secrets_into_payload(
    config: &KeroseneConfig,
    payload: &mut SecretPayload,
) -> bool {
    let mut changed = false;

    for profile in &config.accounts {
        if payload
            .profile_agent_key_for_wallet(&profile.secret_id, &profile.wallet_address)
            .is_none_or(|agent_key| agent_key.trim().is_empty())
            && !profile.agent_key.trim().is_empty()
        {
            changed |= payload.upsert_profile_agent_key_for_wallet(
                &profile.secret_id,
                Some(&profile.wallet_address),
                &profile.agent_key,
            );
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
    if payload.global_x_access_token().trim().is_empty() && !config.x_access_token.trim().is_empty()
    {
        changed |= payload.set_global_x_access_token(&config.x_access_token);
    }
    if payload.global_x_oauth_client_id().trim().is_empty()
        && !config.x_oauth_client_id.trim().is_empty()
    {
        changed |= payload.set_global_x_oauth_client_id(&config.x_oauth_client_id);
    }
    if payload.global_x_refresh_token().trim().is_empty()
        && !config.x_refresh_token.trim().is_empty()
    {
        changed |= payload.set_global_x_refresh_token(&config.x_refresh_token);
    }
    if payload.global_schwab_client_id().trim().is_empty()
        && !config.schwab_client_id.trim().is_empty()
    {
        changed |= payload.set_global_schwab_client_id(&config.schwab_client_id);
    }
    if payload.global_schwab_client_secret().trim().is_empty()
        && !config.schwab_client_secret.trim().is_empty()
    {
        changed |= payload.set_global_schwab_client_secret(&config.schwab_client_secret);
    }
    if payload.global_schwab_access_token().trim().is_empty()
        && !config.schwab_access_token.trim().is_empty()
    {
        changed |= payload.set_global_schwab_access_token(&config.schwab_access_token);
    }
    if payload.global_schwab_refresh_token().trim().is_empty()
        && !config.schwab_refresh_token.trim().is_empty()
    {
        changed |= payload.set_global_schwab_refresh_token(&config.schwab_refresh_token);
    }

    changed
}

pub(super) fn bind_legacy_unbound_profile_keys_to_wallets(
    config: &KeroseneConfig,
    payload: &mut SecretPayload,
) -> bool {
    payload.bind_unbound_profile_agent_keys_to_wallets(&config.accounts)
}

pub(super) fn applied_secret_payload_for_legacy_cleanup(
    config: &KeroseneConfig,
    payload: &SecretPayload,
) -> SecretPayload {
    let mut cleanup_payload = payload.clone();
    cleanup_payload.profiles.retain(|payload_profile| {
        config.accounts.iter().any(|account| {
            account.secret_id == payload_profile.secret_id
                && payload
                    .profile_agent_key_for_wallet(&account.secret_id, &account.wallet_address)
                    .is_some()
        })
    });
    cleanup_payload
}

pub(super) fn apply_secret_payload(config: &mut KeroseneConfig, payload: &SecretPayload) {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }

        profile.agent_key.zeroize();
        if let Some(agent_key) =
            payload.profile_agent_key_for_wallet(&profile.secret_id, &profile.wallet_address)
        {
            profile.agent_key = agent_key.to_string().into();
        } else if payload
            .profile_agent_key_binding_mismatches(&profile.secret_id, &profile.wallet_address)
        {
            push_wallet_binding_mismatch_warning();
        }
        profile.hydromancer_api_key.zeroize();
    }

    config.hydromancer_api_key.zeroize();
    config.hydromancer_api_key = payload.global_hydromancer_api_key().to_string().into();
    config.hyperdash_api_key.zeroize();
    config.hyperdash_api_key = payload.global_hyperdash_api_key().to_string().into();
    config.x_access_token.zeroize();
    config.x_access_token = payload.global_x_access_token().to_string().into();
    config.x_oauth_client_id.zeroize();
    config.x_oauth_client_id = payload.global_x_oauth_client_id().to_string().into();
    config.x_refresh_token.zeroize();
    config.x_refresh_token = payload.global_x_refresh_token().to_string().into();
    config.schwab_client_id.zeroize();
    config.schwab_client_id = payload.global_schwab_client_id().to_string().into();
    config.schwab_client_secret.zeroize();
    config.schwab_client_secret = payload.global_schwab_client_secret().to_string().into();
    config.schwab_access_token.zeroize();
    config.schwab_access_token = payload.global_schwab_access_token().to_string().into();
    config.schwab_refresh_token.zeroize();
    config.schwab_refresh_token = payload.global_schwab_refresh_token().to_string().into();
}

pub(super) fn apply_secret_payload_preserving_missing_plaintext(
    config: &mut KeroseneConfig,
    payload: &SecretPayload,
) {
    for profile in &mut config.accounts {
        if profile.secret_id.is_empty() {
            profile.secret_id = new_secret_id();
        }

        if let Some(agent_key) =
            payload.profile_agent_key_for_wallet(&profile.secret_id, &profile.wallet_address)
        {
            profile.agent_key.zeroize();
            profile.agent_key = agent_key.to_string().into();
        } else if payload
            .profile_agent_key_binding_mismatches(&profile.secret_id, &profile.wallet_address)
        {
            push_wallet_binding_mismatch_warning();
        }
        if !payload.global_hydromancer_api_key().trim().is_empty() {
            profile.hydromancer_api_key.zeroize();
        }
    }

    if !payload.global_hydromancer_api_key().trim().is_empty() {
        config.hydromancer_api_key.zeroize();
        config.hydromancer_api_key = payload.global_hydromancer_api_key().to_string().into();
    }
    if !payload.global_hyperdash_api_key().trim().is_empty() {
        config.hyperdash_api_key.zeroize();
        config.hyperdash_api_key = payload.global_hyperdash_api_key().to_string().into();
    }
    if !payload.global_x_access_token().trim().is_empty() {
        config.x_access_token.zeroize();
        config.x_access_token = payload.global_x_access_token().to_string().into();
    }
    if !payload.global_x_oauth_client_id().trim().is_empty() {
        config.x_oauth_client_id.zeroize();
        config.x_oauth_client_id = payload.global_x_oauth_client_id().to_string().into();
    }
    if !payload.global_x_refresh_token().trim().is_empty() {
        config.x_refresh_token.zeroize();
        config.x_refresh_token = payload.global_x_refresh_token().to_string().into();
    }
    if !payload.global_schwab_client_id().trim().is_empty() {
        config.schwab_client_id.zeroize();
        config.schwab_client_id = payload.global_schwab_client_id().to_string().into();
    }
    if !payload.global_schwab_client_secret().trim().is_empty() {
        config.schwab_client_secret.zeroize();
        config.schwab_client_secret = payload.global_schwab_client_secret().to_string().into();
    }
    if !payload.global_schwab_access_token().trim().is_empty() {
        config.schwab_access_token.zeroize();
        config.schwab_access_token = payload.global_schwab_access_token().to_string().into();
    }
    if !payload.global_schwab_refresh_token().trim().is_empty() {
        config.schwab_refresh_token.zeroize();
        config.schwab_refresh_token = payload.global_schwab_refresh_token().to_string().into();
    }
}

#[cfg(test)]
mod tests;
