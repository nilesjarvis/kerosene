use std::fmt;

use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Captured Agent Key
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapturedAgentKey(Zeroizing<String>);

impl CapturedAgentKey {
    pub(crate) fn new(agent_key: Zeroizing<String>) -> Option<Self> {
        let agent_key = Zeroizing::new(agent_key.trim().to_string());
        (!agent_key.is_empty()).then_some(Self(agent_key))
    }

    pub(crate) fn clone_for_task(&self) -> Zeroizing<String> {
        Zeroizing::new(self.0.trim().to_string())
    }

    pub(crate) fn clear(&mut self) {
        self.0.zeroize();
    }

    #[cfg(test)]
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
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
}
