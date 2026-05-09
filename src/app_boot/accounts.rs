use crate::app_state::TradingTerminal;
use crate::config::{CredentialStorageMode, KeroseneConfig};
use zeroize::Zeroizing;

pub(super) struct BootAccountProfile {
    pub(super) active_account_index: usize,
    pub(super) wallet_address: String,
    pub(super) agent_key: Zeroizing<String>,
    pub(super) hydromancer_key: Zeroizing<String>,
    pub(super) last_persisted_secret_id: Option<String>,
    pub(super) journal_account_key: Option<String>,
    pub(super) show_unlock_credentials_popup: bool,
    pub(super) has_wallet: bool,
}

impl TradingTerminal {
    pub(super) fn boot_account_profile(cfg: &KeroseneConfig) -> BootAccountProfile {
        let active_account_index = if cfg.active_account_index < cfg.accounts.len() {
            cfg.active_account_index
        } else {
            0
        };
        let active_profile = cfg.accounts.get(active_account_index);
        let wallet_address = active_profile
            .map(|profile| profile.wallet_address.clone())
            .unwrap_or_default();
        let agent_key = active_profile
            .map(|profile| profile.agent_key.clone())
            .unwrap_or_default();
        let last_persisted_secret_id = active_profile.map(|profile| profile.secret_id.clone());
        let journal_account_key = active_profile.map(|profile| profile.secret_id.clone());
        let hydromancer_key = cfg.hydromancer_api_key.trim().to_string().into();
        let show_unlock_credentials_popup = cfg.credential_storage_mode
            == CredentialStorageMode::EncryptedConfig
            && cfg.encrypted_secrets.is_some();
        let has_wallet = !wallet_address.is_empty();

        BootAccountProfile {
            active_account_index,
            wallet_address,
            agent_key,
            hydromancer_key,
            last_persisted_secret_id,
            journal_account_key,
            show_unlock_credentials_popup,
            has_wallet,
        }
    }
}
