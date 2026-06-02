use super::super::AccountProfile;

use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretKdfConfig {
    pub algorithm: String,
    pub salt: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedSecretsConfig {
    pub version: u8,
    pub kdf: SecretKdfConfig,
    pub cipher: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSecretPayload {
    pub secret_id: String,
    pub agent_key: Zeroizing<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct GlobalSecretPayload {
    pub hydromancer_api_key: Zeroizing<String>,
    pub hyperdash_api_key: Zeroizing<String>,
    #[serde(default)]
    pub x_bearer_token: Zeroizing<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretPayload {
    pub schema: String,
    pub profiles: Vec<ProfileSecretPayload>,
    pub global: GlobalSecretPayload,
}

impl SecretPayload {
    pub fn from_credentials(
        profiles: &[AccountProfile],
        hydromancer_api_key: &str,
        hyperdash_api_key: &str,
        x_bearer_token: &str,
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
                    agent_key: profile.agent_key.to_string().into(),
                })
                .collect(),
            global: GlobalSecretPayload {
                hydromancer_api_key: hydromancer_api_key.to_string().into(),
                hyperdash_api_key: hyperdash_api_key.to_string().into(),
                x_bearer_token: x_bearer_token.to_string().into(),
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
            && self.global.hydromancer_api_key.trim().is_empty()
            && self.global.hyperdash_api_key.trim().is_empty()
            && self.global.x_bearer_token.trim().is_empty()
    }

    pub fn profile_agent_key(&self, secret_id: &str) -> Option<&str> {
        self.profiles
            .iter()
            .find(|profile| profile.secret_id == secret_id)
            .map(|profile| profile.agent_key.as_str())
    }

    pub fn global_hydromancer_api_key(&self) -> &str {
        &self.global.hydromancer_api_key
    }

    pub fn global_hyperdash_api_key(&self) -> &str {
        &self.global.hyperdash_api_key
    }

    pub fn global_x_bearer_token(&self) -> &str {
        &self.global.x_bearer_token
    }

    pub fn upsert_profile_agent_key(&mut self, secret_id: &str, agent_key: &str) -> bool {
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
            if profile.agent_key.as_str() == agent_key {
                return false;
            }
            profile.agent_key = agent_key.to_string().into();
            return true;
        }

        self.profiles.push(ProfileSecretPayload {
            secret_id: secret_id.to_string(),
            agent_key: agent_key.to_string().into(),
        });
        true
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

    pub fn set_global_x_bearer_token(&mut self, value: &str) -> bool {
        if self.global.x_bearer_token.as_str() == value {
            return false;
        }
        self.global.x_bearer_token = value.to_string().into();
        true
    }
}

#[cfg(test)]
mod tests;
