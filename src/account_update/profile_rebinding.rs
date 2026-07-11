use crate::app_state::{SensitiveString, TradingTerminal};

use zeroize::{Zeroize, Zeroizing};

// ---------------------------------------------------------------------------
// Active Profile Address-Rebind Rollback
// ---------------------------------------------------------------------------

pub(super) struct ActiveProfileAddressRebindRollback {
    profile_index: usize,
    profile_secret_id: String,
    previous_wallet_address: String,
    previous_agent_key: Zeroizing<String>,
    previous_wallet_key_input: SensitiveString,
}

impl TradingTerminal {
    pub(super) fn begin_active_profile_address_rebind(
        &mut self,
        profile_index: usize,
        next_wallet_address: String,
    ) -> Option<ActiveProfileAddressRebindRollback> {
        let profile = self.accounts.get_mut(profile_index)?;
        let profile_secret_id = profile.secret_id.clone();
        // Move both usable key owners out before building the persistence
        // snapshot. Rollback can restore these exact allocations; success owns
        // their immediate scrubbing.
        let previous_wallet_address =
            std::mem::replace(&mut profile.wallet_address, next_wallet_address);
        let previous_agent_key = std::mem::take(&mut profile.agent_key);
        let previous_wallet_key_input = std::mem::take(&mut self.wallet_key_input);

        Some(ActiveProfileAddressRebindRollback {
            profile_index,
            profile_secret_id,
            previous_wallet_address,
            previous_agent_key,
            previous_wallet_key_input,
        })
    }
}

impl ActiveProfileAddressRebindRollback {
    pub(super) fn restore(
        mut self,
        terminal: &mut TradingTerminal,
        previous_wallet_address_input: String,
    ) {
        let profile = terminal
            .accounts
            .get_mut(self.profile_index)
            .expect("active profile disappeared during address-rebind rollback");
        assert!(
            profile.secret_id == self.profile_secret_id,
            "active profile identity changed during address-rebind rollback"
        );
        self.profile_secret_id.zeroize();
        profile.wallet_address = self.previous_wallet_address;
        profile.agent_key = self.previous_agent_key;
        terminal.wallet_key_input = self.previous_wallet_key_input;
        terminal.wallet_address_input = previous_wallet_address_input;
    }

    pub(super) fn scrub_after_commit(mut self) {
        self.profile_secret_id.zeroize();
        self.previous_wallet_address.zeroize();
        self.previous_agent_key.zeroize();
        self.previous_wallet_key_input.zeroize();
    }
}
