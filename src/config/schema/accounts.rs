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
    /// Parent account for a subaccount; `wallet_address` remains the effective account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_address: Option<String>,
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
            .field("name", &self.name)
            .field("wallet_address", &"<redacted>")
            .field(
                "master_address",
                &self.master_address.as_ref().map(|_| "<redacted>"),
            )
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
        let profile = AccountProfile {
            master_address: Some("0x1234567890123456789012345678901234567890".to_string()),
            secret_id: "acct-secret-id".to_string(),
            name: "Trading Profile".to_string(),
            wallet_address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string(),
            agent_key: "agent-secret".to_string().into(),
            hydromancer_api_key: "hydro-secret".to_string().into(),
        };

        let rendered = format!("{profile:?}");

        assert!(rendered.contains("Trading Profile"));
        assert!(rendered.contains("<redacted>"));
        for secret in [
            "acct-secret-id",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "0x1234567890123456789012345678901234567890",
            "agent-secret",
            "hydro-secret",
        ] {
            assert!(!rendered.contains(secret), "debug output leaked {secret}");
        }
    }

    #[test]
    fn account_profile_subaccount_metadata_round_trips_and_defaults_for_legacy() {
        let legacy = serde_json::json!({
            "secret_id": "acct-a",
            "name": "Trading",
            "wallet_address": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        });
        let mut profile: AccountProfile = serde_json::from_value(legacy).expect("legacy profile");
        assert!(profile.master_address.is_none());
        assert!(
            serde_json::to_value(&profile)
                .expect("profile JSON")
                .get("master_address")
                .is_none()
        );

        profile.master_address = Some("0x1234567890123456789012345678901234567890".to_string());
        profile.agent_key = "agent-secret".to_string().into();
        let encoded = serde_json::to_value(&profile).expect("subaccount JSON");
        assert!(encoded.get("agent_key").is_none());
        let decoded: AccountProfile = serde_json::from_value(encoded).expect("subaccount profile");
        assert_eq!(decoded.master_address, profile.master_address);
        assert_eq!(decoded.wallet_address, profile.wallet_address);
        assert!(decoded.agent_key.is_empty());
    }
}
