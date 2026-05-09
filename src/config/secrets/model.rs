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
    ) -> Self {
        Self {
            schema: SECRET_PAYLOAD_SCHEMA.to_string(),
            profiles: profiles
                .iter()
                .filter(|profile| !profile.secret_id.trim().is_empty())
                .map(|profile| ProfileSecretPayload {
                    secret_id: profile.secret_id.clone(),
                    agent_key: profile.agent_key.to_string().into(),
                })
                .collect(),
            global: GlobalSecretPayload {
                hydromancer_api_key: hydromancer_api_key.to_string().into(),
                hyperdash_api_key: hyperdash_api_key.to_string().into(),
            },
        }
    }
}
