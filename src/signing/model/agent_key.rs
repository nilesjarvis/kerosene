use std::fmt;

use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Captured Agent Key
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapturedAgentKey {
    agent_key: Zeroizing<String>,
    vault_address: Option<String>,
}

impl CapturedAgentKey {
    #[cfg(test)]
    pub(crate) fn new(agent_key: Zeroizing<String>) -> Option<Self> {
        Self::for_account(agent_key, None).ok()
    }

    /// Capture the signing credential and target together before asynchronous
    /// work starts. An invalid subaccount must never fall back to the master.
    pub(crate) fn for_account(
        agent_key: Zeroizing<String>,
        vault_address: Option<&str>,
    ) -> Result<Self, String> {
        let vault_address = vault_address
            .map(|address| {
                let address = address.trim();
                let valid = address.strip_prefix("0x").is_some_and(|hex| {
                    hex.len() == 40 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                });
                if !valid {
                    return Err(
                        "Invalid subaccount address: expected 0x and 40 hex digits".to_string()
                    );
                }
                Ok(address.to_ascii_lowercase())
            })
            .transpose()?;
        let agent_key = Zeroizing::new(agent_key.trim().to_string());
        if agent_key.is_empty() {
            return Err("Agent key is required".to_string());
        }
        Ok(Self {
            agent_key,
            vault_address,
        })
    }

    pub(crate) fn clone_for_task(&self) -> Self {
        self.clone()
    }

    pub(crate) fn vault_address(&self) -> Option<&str> {
        self.vault_address.as_deref()
    }

    pub(in crate::signing) fn private_key(&self) -> &str {
        self.agent_key.as_str()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.agent_key.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.agent_key.zeroize();
        self.vault_address = None;
    }

    #[cfg(test)]
    pub(crate) fn as_str(&self) -> &str {
        self.agent_key.as_str()
    }
}

impl fmt::Debug for CapturedAgentKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapturedAgentKey(<redacted>)")
    }
}

#[cfg(test)]
impl From<String> for CapturedAgentKey {
    fn from(value: String) -> Self {
        Self::new(Zeroizing::new(value)).expect("test captured agent key should be non-empty")
    }
}

#[cfg(test)]
impl From<Zeroizing<String>> for CapturedAgentKey {
    fn from(value: Zeroizing<String>) -> Self {
        Self::new(value).expect("test captured agent key should be non-empty")
    }
}

#[cfg(test)]
impl From<&str> for CapturedAgentKey {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUBACCOUNT: &str = "0xabcdef0123456789abcdef0123456789abcdef01";

    #[test]
    fn captured_agent_key_trims_task_clone() {
        let key = CapturedAgentKey::new(Zeroizing::new("  agent-secret  ".to_string()))
            .expect("valid key");

        assert_eq!(key.clone_for_task().as_str(), "agent-secret");
    }

    #[test]
    fn captured_agent_key_rejects_empty_values() {
        assert!(CapturedAgentKey::new(Zeroizing::new("   ".to_string())).is_none());
    }

    #[test]
    fn captured_agent_key_clear_removes_task_clone_value() {
        let mut key =
            CapturedAgentKey::new(Zeroizing::new("agent-secret".to_string())).expect("valid key");

        key.clear();

        assert!(key.as_str().is_empty());
        assert!(key.clone_for_task().is_empty());
    }

    #[test]
    fn captured_agent_key_debug_redacts_value() {
        let key =
            CapturedAgentKey::new(Zeroizing::new("agent-secret".to_string())).expect("valid key");

        let rendered = format!("{key:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("agent-secret"));
    }

    #[test]
    fn captured_agent_key_preserves_subaccount_across_task_clones() {
        let key = CapturedAgentKey::for_account(
            Zeroizing::new(" agent-secret ".to_string()),
            Some(" 0xABCDEF0123456789ABCDEF0123456789ABCDEF01 "),
        )
        .expect("valid subaccount");

        let task_key = key.clone_for_task();

        assert_eq!(task_key.as_str(), "agent-secret");
        assert_eq!(task_key.vault_address(), Some(SUBACCOUNT));
        assert_eq!(key, task_key);
    }

    #[test]
    fn captured_agent_key_master_has_no_subaccount_target() {
        let key = CapturedAgentKey::new(Zeroizing::new("agent-secret".to_string()))
            .expect("valid master key");

        assert_eq!(key.vault_address(), None);
        assert_eq!(key.clone_for_task().vault_address(), None);
    }

    #[test]
    fn captured_agent_key_rejects_invalid_subaccounts_without_echoing_input() {
        for address in [
            "",
            "   ",
            "0x",
            "0x1234",
            "abcdef0123456789abcdef0123456789abcdef01",
            "0xabcdef0123456789abcdef0123456789abcdef0123",
            "0xabcdef0123456789abcdef0123456789abcdef0z",
        ] {
            let error = CapturedAgentKey::for_account(
                Zeroizing::new("agent-secret".to_string()),
                Some(address),
            )
            .expect_err("invalid subaccount must not fall back to master");

            assert!(error.starts_with("Invalid subaccount address:"));
            assert!(!error.contains("agent-secret"));
            if address.len() > 2 {
                assert!(!error.contains(address));
            }
        }
    }

    #[test]
    fn captured_agent_key_subaccount_requires_non_empty_key() {
        assert!(
            CapturedAgentKey::for_account(Zeroizing::new("   ".to_string()), Some(SUBACCOUNT))
                .is_err()
        );
    }

    #[test]
    fn captured_agent_key_clear_removes_subaccount_and_preserves_existing_task() {
        let mut key = CapturedAgentKey::for_account(
            Zeroizing::new("agent-secret".to_string()),
            Some(SUBACCOUNT),
        )
        .expect("valid subaccount");
        let task_key = key.clone_for_task();

        key.clear();

        assert!(key.is_empty());
        assert_eq!(key.vault_address(), None);
        assert!(key.clone_for_task().is_empty());
        assert_eq!(key.clone_for_task().vault_address(), None);
        assert_eq!(task_key.as_str(), "agent-secret");
        assert_eq!(task_key.vault_address(), Some(SUBACCOUNT));
    }

    #[test]
    fn captured_agent_key_debug_redacts_subaccount() {
        let key = CapturedAgentKey::for_account(
            Zeroizing::new("agent-secret".to_string()),
            Some(SUBACCOUNT),
        )
        .expect("valid subaccount");

        let rendered = format!("{key:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("agent-secret"));
        assert!(!rendered.contains(SUBACCOUNT));
    }
}
