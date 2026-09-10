use super::{AddAccountTarget, DiscoveredSubaccount, SubaccountDiscoveryRequest};
use crate::app_state::SensitiveString;
use crate::wallet_state::address_book::normalize_wallet_address_value;

use iced::window;

// ---------------------------------------------------------------------------
// Add Account Window
// ---------------------------------------------------------------------------

/// Draft state for the dedicated add-account window. Nothing here touches the
/// account list or credential storage until the user submits; closing the
/// window drops the draft (the key input zeroizes on drop).
pub(crate) struct AddAccountWindowState {
    pub(crate) window_id: window::Id,
    pub(crate) name_input: String,
    pub(crate) address_input: String,
    pub(crate) key_input: SensitiveString,
    pub(crate) switch_on_add: bool,
    pub(crate) error: Option<String>,
    pub(crate) target: Option<AddAccountTarget>,
    pub(crate) subaccounts: Vec<DiscoveredSubaccount>,
    pub(crate) discovery_request: Option<SubaccountDiscoveryRequest>,
    pub(crate) discovery_generation: u64,
    pub(crate) discovered_master: Option<String>,
    pub(crate) discovery_error: Option<String>,
}

impl AddAccountWindowState {
    pub(crate) fn new(window_id: window::Id) -> Self {
        Self {
            window_id,
            name_input: String::new(),
            address_input: String::new(),
            key_input: SensitiveString::default(),
            switch_on_add: true,
            error: None,
            target: Some(AddAccountTarget::Master),
            subaccounts: Vec::new(),
            discovery_request: None,
            discovery_generation: 0,
            discovered_master: None,
            discovery_error: None,
        }
    }

    pub(crate) fn invalidate_subaccounts(&mut self) {
        self.discovery_request = None;
        self.subaccounts.clear();
        self.discovered_master = None;
        self.discovery_error = None;
        if matches!(self.target, Some(AddAccountTarget::Subaccount(_))) {
            self.target = None;
        }
    }

    pub(crate) fn default_profile_name(&self, account_number: usize) -> String {
        if let Some(AddAccountTarget::Subaccount(child)) = &self.target {
            let name = child.name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        format!("Account {account_number}")
    }

    pub(crate) fn profile_name(&self, account_number: usize) -> String {
        let name = self.name_input.trim();
        if name.is_empty() {
            self.default_profile_name(account_number)
        } else {
            name.to_string()
        }
    }

    pub(crate) fn selected_addresses(&self) -> Result<(String, Option<String>), String> {
        let master = normalize_wallet_address_value(&self.address_input).ok_or_else(|| {
            "Enter a valid master account address (0x followed by 40 hex characters).".to_string()
        })?;
        match &self.target {
            Some(AddAccountTarget::Master) => Ok((master, None)),
            Some(AddAccountTarget::Subaccount(child))
                if self.discovery_request.is_none()
                    && self.discovered_master.as_deref() == Some(master.as_str())
                    && child.master_address.as_str() == master
                    && child.address.as_str() != master
                    && normalize_wallet_address_value(&child.address).as_deref()
                        == Some(child.address.as_str())
                    && self.subaccounts.contains(child) =>
            {
                Ok((child.address.as_str().to_string(), Some(master)))
            }
            _ => Err("Select an account again before adding this profile.".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &str = "0xabc0000000000000000000000000000000000000";
    const CHILD: &str = "0xdef0000000000000000000000000000000000000";

    fn child_state() -> AddAccountWindowState {
        let mut state = AddAccountWindowState::new(window::Id::unique());
        let child = DiscoveredSubaccount {
            name: "Strategy".to_string(),
            address: CHILD.into(),
            master_address: MASTER.into(),
        };
        state.address_input = MASTER.to_string();
        state.discovered_master = Some(MASTER.to_string());
        state.target = Some(AddAccountTarget::Subaccount(child.clone()));
        state.subaccounts = vec![child];
        state
    }

    #[test]
    fn child_name_is_the_default_but_explicit_profile_names_take_precedence() {
        let mut state = child_state();
        assert_eq!(state.default_profile_name(2), "Strategy");
        assert_eq!(state.profile_name(2), state.default_profile_name(2));
        state.name_input = "  My strategy  ".to_string();
        assert_eq!(state.profile_name(2), "My strategy");
        assert_eq!(state.default_profile_name(2), "Strategy");
        state.name_input = "  ".to_string();
        assert_eq!(state.profile_name(2), "Strategy");
        state.invalidate_subaccounts();
        assert_eq!(state.profile_name(2), "Account 2");
        state.target = Some(AddAccountTarget::Master);
        assert_eq!(state.default_profile_name(2), "Account 2");
        assert_eq!(state.profile_name(2), "Account 2");
    }

    #[test]
    fn selected_addresses_require_current_valid_parent_and_child_membership() {
        let state = child_state();
        assert_eq!(
            state.selected_addresses().expect("valid child selection"),
            (CHILD.to_string(), Some(MASTER.to_string()))
        );
        let mut state = child_state();
        state.subaccounts.clear();
        assert!(state.selected_addresses().is_err());
        let mut state = child_state();
        state.discovered_master = None;
        assert!(state.selected_addresses().is_err());
        let mut state = child_state();
        state.address_input = CHILD.to_string();
        assert!(state.selected_addresses().is_err());
        let mut state = child_state();
        state.discovery_request = Some(SubaccountDiscoveryRequest {
            window_id: state.window_id,
            generation: 1,
            master_address: MASTER.into(),
        });
        assert!(state.selected_addresses().is_err());
    }

    #[test]
    fn invalid_child_relationship_never_becomes_a_master_selection() {
        for (child_address, master_address) in [
            ("invalid", MASTER),
            (CHILD, "invalid"),
            (CHILD, CHILD),
            (MASTER, MASTER),
        ] {
            let mut state = child_state();
            let child = DiscoveredSubaccount {
                name: "Strategy".to_string(),
                address: child_address.into(),
                master_address: master_address.into(),
            };
            state.subaccounts = vec![child.clone()];
            state.target = Some(AddAccountTarget::Subaccount(child));
            assert!(state.selected_addresses().is_err());
        }
    }

    #[test]
    fn discovery_invalidation_preserves_only_an_explicit_master_selection() {
        let mut state = child_state();
        state.invalidate_subaccounts();
        assert!(state.target.is_none());
        assert!(state.selected_addresses().is_err());
        state.target = Some(AddAccountTarget::Master);
        state.invalidate_subaccounts();
        assert_eq!(
            state
                .selected_addresses()
                .expect("explicit master selection"),
            (MASTER.to_string(), None)
        );
    }
}
