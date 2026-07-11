use super::new_secret_id;
use serde::{Deserialize, Deserializer, Serialize};
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
            .field("secret_id", &"<redacted>")
            .field("name", &"<redacted>")
            .field("wallet_address", &"<redacted>")
            .field("agent_key", &"<redacted>")
            .field("hydromancer_api_key", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
pub enum CredentialStorageMode {
    #[default]
    OsKeychain,
    EncryptedConfig,
}

impl CredentialStorageMode {
    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "OsKeychain" => Some(Self::OsKeychain),
            "EncryptedConfig" => Some(Self::EncryptedConfig),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for CredentialStorageMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let Some(value) = value.as_str() else {
            crate::config::push_config_warning(
                "Invalid credential storage mode in config; using OS Keychain".to_string(),
            );
            return Ok(Self::default());
        };

        Ok(Self::from_config_value(value).unwrap_or_else(|| {
            crate::config::push_config_warning(
                "Unknown credential storage mode in config; using OS Keychain".to_string(),
            );
            Self::default()
        }))
    }
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

#[cfg(test)]
mod tests {
    use super::AccountProfile;

    #[test]
    fn account_profile_debug_redacts_secret_identity_metadata() {
        const NAME: &str = "private-account-profile-name-sentinel";
        let profile = AccountProfile {
            secret_id: "acct-secret-id".to_string(),
            name: NAME.to_string(),
            wallet_address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string(),
            agent_key: "agent-secret".to_string().into(),
            hydromancer_api_key: "hydro-secret".to_string().into(),
        };
        let wire_before = serde_json::to_value(&profile).expect("serialize account profile");

        let rendered = format!("{profile:?}");

        assert!(rendered.contains("<redacted>"));
        for secret in [
            NAME,
            "acct-secret-id",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "agent-secret",
            "hydro-secret",
        ] {
            assert!(!rendered.contains(secret), "debug output leaked {secret}");
        }
        assert_eq!(profile.name, NAME);
        assert_eq!(
            serde_json::to_value(&profile).expect("serialize account profile after formatting"),
            wire_before
        );
    }
}
