use std::fmt;

use zeroize::{Zeroize, Zeroizing};

use crate::builder_codes::{BuilderCodeConfig, BuilderFee, normalize_builder_address};

// ---------------------------------------------------------------------------
// Captured Agent Key
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapturedAgentKey {
    agent_key: Zeroizing<String>,
    vault_address: Option<String>,
    builder: Result<Option<BuilderContext>, String>,
}

#[derive(Clone, PartialEq, Eq)]
struct BuilderContext {
    fee: BuilderFee,
    account_address: String,
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
            builder: Ok(None),
        })
    }

    /// Preserve invalid enabled configuration as a placement rejection rather
    /// than silently dropping attribution. Cancels and leverage remain available.
    pub(crate) fn with_builder_config(
        mut self,
        config: &BuilderCodeConfig,
        account_address: &str,
    ) -> Self {
        self.builder = config
            .resolve()
            .and_then(|fee| capture_builder(fee, account_address));
        self
    }

    #[cfg(test)]
    pub(crate) fn with_builder(
        mut self,
        builder: Option<BuilderFee>,
        account_address: &str,
    ) -> Result<Self, String> {
        self.builder = Ok(capture_builder(builder, account_address)?);
        Ok(self)
    }

    pub(crate) fn builder_fee(&self) -> Option<&BuilderFee> {
        self.builder
            .as_ref()
            .ok()?
            .as_ref()
            .map(|context| &context.fee)
    }

    #[cfg(test)]
    pub(crate) fn builder_account_address(&self) -> Option<&str> {
        self.builder
            .as_ref()
            .ok()?
            .as_ref()
            .map(|context| context.account_address.as_str())
    }

    pub(crate) fn builder_configured(&self) -> bool {
        !matches!(self.builder, Ok(None))
    }

    pub(crate) fn validate_builder_routing(&self) -> Result<Option<(&str, &BuilderFee)>, String> {
        let context = self.builder.as_ref().map_err(Clone::clone)?;
        if self.builder_configured() && self.vault_address.is_some() {
            return Err(
                "Builder codes are not yet supported for subaccount or vault orders".to_string(),
            );
        }
        Ok(context
            .as_ref()
            .map(|context| (context.account_address.as_str(), &context.fee)))
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
        self.builder = Ok(None);
    }

    #[cfg(test)]
    pub(crate) fn as_str(&self) -> &str {
        self.agent_key.as_str()
    }
}

fn capture_builder(
    fee: Option<BuilderFee>,
    account_address: &str,
) -> Result<Option<BuilderContext>, String> {
    fee.map(|fee| {
        let account_address = normalize_builder_address(account_address)
            .map_err(|_| "A valid account address is required for builder approval".to_string())?;
        Ok(BuilderContext {
            fee,
            account_address,
        })
    })
    .transpose()
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
    const BUILDER: &str = "0x1111111111111111111111111111111111111111";

    fn builder_config() -> BuilderCodeConfig {
        BuilderCodeConfig {
            enabled: true,
            address: BUILDER.to_string(),
            fee_tenths_bps: 37,
        }
    }

    #[test]
    fn captured_builder_retains_exact_account_and_fee_after_settings_change() {
        let mut config = builder_config();
        let key = CapturedAgentKey::from("agent-secret").with_builder_config(
            &config,
            &format!(" 0x{} ", SUBACCOUNT[2..].to_ascii_uppercase()),
        );
        let task_key = key.clone_for_task();
        config.address = SUBACCOUNT.to_string();
        config.fee_tenths_bps = 100;
        config.enabled = false;

        let (account, builder) = task_key
            .validate_builder_routing()
            .expect("valid routing")
            .expect("enabled builder");
        assert_eq!(account, SUBACCOUNT);
        assert_eq!(builder.address(), BUILDER);
        assert_eq!(builder.fee_tenths_bps(), 37);
        assert_eq!(task_key.builder_account_address(), Some(SUBACCOUNT));
        assert_eq!(task_key, key);
        assert_eq!(config.resolve().expect("now disabled"), None);
        let debug = format!("{task_key:?}");
        assert!(!debug.contains(SUBACCOUNT));
        assert!(!debug.contains(BUILDER));
        assert!(!debug.contains("agent-secret"));
    }

    #[test]
    fn invalid_enabled_builder_stays_blocked_across_task_clones() {
        let invalid = BuilderCodeConfig {
            enabled: true,
            ..BuilderCodeConfig::default()
        };
        let key = CapturedAgentKey::from("agent-secret").with_builder_config(&invalid, SUBACCOUNT);
        assert!(key.builder_configured());
        assert!(key.builder_fee().is_none());
        assert!(key.validate_builder_routing().is_err());
        assert!(key.clone_for_task().validate_builder_routing().is_err());
        assert!(!key.is_empty());
    }

    #[test]
    fn builder_clear_drops_routing_without_changing_inflight_task_capture() {
        let mut key = CapturedAgentKey::from("agent-secret")
            .with_builder_config(&builder_config(), SUBACCOUNT);
        let captured = key.clone_for_task();
        key.clear();
        assert!(!key.builder_configured());
        assert_eq!(key.builder_fee(), None);
        assert_eq!(key.builder_account_address(), None);
        assert!(captured.builder_configured());
        assert_eq!(captured.builder_account_address(), Some(SUBACCOUNT));
    }

    #[test]
    fn disabled_builder_allows_invalid_account_but_enabled_builder_cannot() {
        let key = CapturedAgentKey::from("agent-secret")
            .with_builder_config(&BuilderCodeConfig::default(), "bad-account");
        assert!(!key.builder_configured());
        assert_eq!(
            key.validate_builder_routing().expect("disabled routing"),
            None
        );

        let key = CapturedAgentKey::from("agent-secret")
            .with_builder_config(&builder_config(), "bad-account");
        let error = key
            .validate_builder_routing()
            .expect_err("invalid account blocks");
        assert!(!error.contains("bad-account"));
        assert!(!error.contains(BUILDER));
    }

    #[test]
    fn subaccount_capture_preserves_cancel_context_but_blocks_builder_placement() {
        let key = CapturedAgentKey::for_account(
            Zeroizing::new("agent-secret".to_string()),
            Some(SUBACCOUNT),
        )
        .expect("valid subaccount")
        .with_builder_config(&builder_config(), SUBACCOUNT);
        assert!(!key.is_empty());
        assert_eq!(key.vault_address(), Some(SUBACCOUNT));
        assert!(key.builder_fee().is_some());
        assert!(
            key.validate_builder_routing()
                .expect_err("unsupported subaccount")
                .contains("subaccount")
        );
    }

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
