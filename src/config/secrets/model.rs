use super::super::AccountProfile;

use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

pub(super) const SECRET_PAYLOAD_SCHEMA: &str = "kerosene.secrets.v1";
pub(super) const ENCRYPTED_SECRETS_VERSION: u8 = 1;
pub(super) const ENCRYPTED_SECRETS_CIPHER: &str = "xchacha20poly1305";
pub(super) const SECRET_SALT_LEN: usize = 16;
pub(super) const SECRET_NONCE_LEN: usize = 24;
pub(super) const SECRET_KEY_LEN: usize = 32;

pub(super) const DEFAULT_ARGON2_MEMORY_KIB: u32 = 64 * 1024;
pub(super) const DEFAULT_ARGON2_ITERATIONS: u32 = 3;
pub(super) const DEFAULT_ARGON2_LANES: u32 = 1;

pub(super) fn redacted_secret_payload_parse_error(
    context: &str,
    error: serde_json::Error,
) -> String {
    let kind = match error.classify() {
        serde_json::error::Category::Io => "secret payload I/O failed",
        serde_json::error::Category::Syntax => "invalid secret payload JSON",
        serde_json::error::Category::Data => "secret payload data did not match expected shape",
        serde_json::error::Category::Eof => "truncated secret payload JSON",
    };
    format!(
        "{context}: {kind} at line {}, column {}",
        error.line(),
        error.column()
    )
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretKdfConfig {
    pub algorithm: String,
    pub salt: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
}

impl fmt::Debug for SecretKdfConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretKdfConfig")
            .field("algorithm", &self.algorithm)
            .field("salt", &"<redacted>")
            .field("memory_kib", &self.memory_kib)
            .field("iterations", &self.iterations)
            .field("lanes", &self.lanes)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedSecretsConfig {
    pub version: u8,
    pub kdf: SecretKdfConfig,
    pub cipher: String,
    pub nonce: String,
    pub ciphertext: String,
}

impl fmt::Debug for EncryptedSecretsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedSecretsConfig")
            .field("version", &self.version)
            .field("kdf", &self.kdf)
            .field("cipher", &self.cipher)
            .field("nonce", &"<redacted>")
            .field("ciphertext", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSecretPayload {
    pub secret_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    pub agent_key: Zeroizing<String>,
}

impl fmt::Debug for ProfileSecretPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileSecretPayload")
            .field("secret_id", &"<redacted>")
            .field(
                "wallet_address",
                &self.wallet_address.as_ref().map(|_| "<redacted>"),
            )
            .field("agent_key", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct GlobalSecretPayload {
    #[serde(default)]
    pub hydromancer_api_key: Zeroizing<String>,
    #[serde(default)]
    pub hyperdash_api_key: Zeroizing<String>,
    #[serde(default)]
    pub x_access_token: Zeroizing<String>,
    #[serde(default)]
    pub x_oauth_client_id: Zeroizing<String>,
    #[serde(default)]
    pub x_refresh_token: Zeroizing<String>,
}

impl fmt::Debug for GlobalSecretPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlobalSecretPayload")
            .field("hydromancer_api_key", &"<redacted>")
            .field("hyperdash_api_key", &"<redacted>")
            .field("x_access_token", &"<redacted>")
            .field("x_oauth_client_id", &"<redacted>")
            .field("x_refresh_token", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretPayload {
    pub schema: String,
    #[serde(default)]
    pub profiles: Vec<ProfileSecretPayload>,
    #[serde(default)]
    pub global: GlobalSecretPayload,
}

impl fmt::Debug for SecretPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretPayload")
            .field("schema", &self.schema)
            .field("profiles", &self.profiles)
            .field("global", &self.global)
            .finish()
    }
}

impl SecretPayload {
    pub(crate) fn normalize_wallet_address(input: &str) -> Option<String> {
        let address = input.trim().to_lowercase();
        let hex = address.strip_prefix("0x")?;
        (hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(address)
    }

    #[cfg(test)]
    pub fn from_credentials(
        profiles: &[AccountProfile],
        hydromancer_api_key: &str,
        hyperdash_api_key: &str,
    ) -> Self {
        Self::from_credentials_with_x(profiles, hydromancer_api_key, hyperdash_api_key, "")
    }

    #[cfg(test)]
    pub fn from_credentials_with_x(
        profiles: &[AccountProfile],
        hydromancer_api_key: &str,
        hyperdash_api_key: &str,
        x_access_token: &str,
    ) -> Self {
        Self::from_credentials_with_x_oauth(
            profiles,
            hydromancer_api_key,
            hyperdash_api_key,
            x_access_token,
            "",
            "",
        )
    }

    pub fn from_credentials_with_x_oauth(
        profiles: &[AccountProfile],
        hydromancer_api_key: &str,
        hyperdash_api_key: &str,
        x_access_token: &str,
        x_oauth_client_id: &str,
        x_refresh_token: &str,
    ) -> Self {
        Self {
            schema: SECRET_PAYLOAD_SCHEMA.to_string(),
            profiles: profiles
                .iter()
                .filter(|profile| {
                    !profile.secret_id.trim().is_empty() && !profile.agent_key.trim().is_empty()
                })
                .map(|profile| ProfileSecretPayload {
                    secret_id: profile.secret_id.clone(),
                    wallet_address: Self::normalize_wallet_address(&profile.wallet_address),
                    agent_key: profile.agent_key.to_string().into(),
                })
                .collect(),
            global: GlobalSecretPayload {
                hydromancer_api_key: hydromancer_api_key.to_string().into(),
                hyperdash_api_key: hyperdash_api_key.to_string().into(),
                x_access_token: x_access_token.to_string().into(),
                x_oauth_client_id: x_oauth_client_id.to_string().into(),
                x_refresh_token: x_refresh_token.to_string().into(),
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
            && self.global.hydromancer_api_key.trim().is_empty()
            && self.global.hyperdash_api_key.trim().is_empty()
            && self.global.x_access_token.trim().is_empty()
            && self.global.x_oauth_client_id.trim().is_empty()
            && self.global.x_refresh_token.trim().is_empty()
    }

    #[cfg(test)]
    pub fn profile_agent_key(&self, secret_id: &str) -> Option<&str> {
        self.profiles
            .iter()
            .find(|profile| profile.secret_id == secret_id)
            .map(|profile| profile.agent_key.as_str())
    }

    fn profile_secret_for_wallet(
        &self,
        secret_id: &str,
        wallet_address: &str,
    ) -> Option<&ProfileSecretPayload> {
        let normalized_wallet = Self::normalize_wallet_address(wallet_address);
        if let Some(wallet_address) = normalized_wallet.as_deref()
            && let Some(profile) = self.profiles.iter().find(|profile| {
                profile.secret_id == secret_id
                    && profile.wallet_address.as_deref() == Some(wallet_address)
            })
        {
            return Some(profile);
        }

        self.profiles.iter().find(|profile| {
            profile.secret_id == secret_id && profile.wallet_address_matches(wallet_address)
        })
    }

    pub fn profile_agent_key_for_wallet(
        &self,
        secret_id: &str,
        wallet_address: &str,
    ) -> Option<&str> {
        self.profile_secret_for_wallet(secret_id, wallet_address)
            .map(|profile| profile.agent_key.as_str())
    }

    pub fn profile_agent_key_binding_mismatches(
        &self,
        secret_id: &str,
        wallet_address: &str,
    ) -> bool {
        self.profiles
            .iter()
            .any(|profile| profile.secret_id == secret_id)
            && self
                .profile_secret_for_wallet(secret_id, wallet_address)
                .is_none()
    }

    pub fn global_hydromancer_api_key(&self) -> &str {
        &self.global.hydromancer_api_key
    }

    pub fn global_hyperdash_api_key(&self) -> &str {
        &self.global.hyperdash_api_key
    }

    pub fn global_x_access_token(&self) -> &str {
        &self.global.x_access_token
    }

    pub fn global_x_oauth_client_id(&self) -> &str {
        &self.global.x_oauth_client_id
    }

    pub fn global_x_refresh_token(&self) -> &str {
        &self.global.x_refresh_token
    }

    #[cfg(test)]
    pub fn upsert_profile_agent_key(&mut self, secret_id: &str, agent_key: &str) -> bool {
        self.upsert_profile_agent_key_for_wallet(secret_id, None, agent_key)
    }

    pub fn upsert_profile_agent_key_for_wallet(
        &mut self,
        secret_id: &str,
        wallet_address: Option<&str>,
        agent_key: &str,
    ) -> bool {
        let secret_id = secret_id.trim();
        if secret_id.is_empty() {
            return false;
        }

        if agent_key.trim().is_empty() {
            return self.remove_profile(secret_id);
        }

        if let Some(profile) = self
            .profiles
            .iter_mut()
            .find(|profile| profile.secret_id == secret_id)
        {
            let normalized_wallet = wallet_address.and_then(Self::normalize_wallet_address);
            if profile.agent_key.as_str() == agent_key
                && profile.wallet_address == normalized_wallet
            {
                return false;
            }
            profile.wallet_address = normalized_wallet;
            profile.agent_key = agent_key.to_string().into();
            return true;
        }

        self.profiles.push(ProfileSecretPayload {
            secret_id: secret_id.to_string(),
            wallet_address: wallet_address.and_then(Self::normalize_wallet_address),
            agent_key: agent_key.to_string().into(),
        });
        true
    }

    pub fn bind_unbound_profile_agent_keys_to_wallets(
        &mut self,
        profiles: &[AccountProfile],
    ) -> bool {
        let mut changed = false;
        for profile in &mut self.profiles {
            let secret_id = profile.secret_id.trim();
            if secret_id.is_empty()
                || profile.wallet_address.is_some()
                || profile.agent_key.trim().is_empty()
            {
                continue;
            }

            let mut matching_wallets = profiles.iter().filter_map(|account| {
                (account.secret_id.trim() == secret_id)
                    .then(|| Self::normalize_wallet_address(&account.wallet_address))
                    .flatten()
            });
            let Some(wallet_address) = matching_wallets.next() else {
                continue;
            };
            if matching_wallets.next().is_some() {
                continue;
            }

            profile.wallet_address = Some(wallet_address);
            changed = true;
        }
        changed
    }

    pub fn remove_profile(&mut self, secret_id: &str) -> bool {
        let original_len = self.profiles.len();
        self.profiles
            .retain(|profile| profile.secret_id != secret_id.trim());
        self.profiles.len() != original_len
    }

    pub fn set_global_hydromancer_api_key(&mut self, value: &str) -> bool {
        if self.global.hydromancer_api_key.as_str() == value {
            return false;
        }
        self.global.hydromancer_api_key = value.to_string().into();
        true
    }

    pub fn set_global_hyperdash_api_key(&mut self, value: &str) -> bool {
        if self.global.hyperdash_api_key.as_str() == value {
            return false;
        }
        self.global.hyperdash_api_key = value.to_string().into();
        true
    }

    pub fn set_global_x_access_token(&mut self, value: &str) -> bool {
        if self.global.x_access_token.as_str() == value {
            return false;
        }
        self.global.x_access_token = value.to_string().into();
        true
    }

    pub fn set_global_x_oauth_client_id(&mut self, value: &str) -> bool {
        if self.global.x_oauth_client_id.as_str() == value {
            return false;
        }
        self.global.x_oauth_client_id = value.to_string().into();
        true
    }

    pub fn set_global_x_refresh_token(&mut self, value: &str) -> bool {
        if self.global.x_refresh_token.as_str() == value {
            return false;
        }
        self.global.x_refresh_token = value.to_string().into();
        true
    }
}

impl ProfileSecretPayload {
    fn wallet_address_matches(&self, wallet_address: &str) -> bool {
        let Some(saved_address) = self.wallet_address.as_deref() else {
            return true;
        };
        SecretPayload::normalize_wallet_address(wallet_address).as_deref() == Some(saved_address)
    }
}

#[cfg(test)]
mod tests;
