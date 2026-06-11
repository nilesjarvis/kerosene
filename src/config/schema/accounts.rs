use super::new_secret_id;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Account and Credential Schema
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountProfile {
    #[serde(default = "new_secret_id")]
    pub secret_id: String,
    pub name: String,
    pub wallet_address: String,
    #[serde(default, skip_serializing)]
    pub agent_key: Zeroizing<String>,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub hydromancer_api_key: Zeroizing<String>,
}

impl fmt::Debug for AccountProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountProfile")
            .field("secret_id", &self.secret_id)
            .field("name", &self.name)
            .field("wallet_address", &self.wallet_address)
            .field("agent_key", &"<redacted>")
            .field("hydromancer_api_key", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CredentialStorageMode {
    #[default]
    OsKeychain,
    EncryptedConfig,
}

impl std::fmt::Display for CredentialStorageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::OsKeychain => "OS Keychain",
            Self::EncryptedConfig => "Encrypted Config",
        };
        f.write_str(label)
    }
}
